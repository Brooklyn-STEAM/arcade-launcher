use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, Manager};
use tower_http::cors::CorsLayer;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEntry {
    pub id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub thumbnail_path: String,
    pub executable_path: String,
    pub version: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub admin_pin: String,
    pub mame_path: String,
    pub mame_args: Vec<String>,
    pub games_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            admin_pin: "1234".to_string(),
            mame_path: "C:\\mame\\mame64.exe".to_string(),
            mame_args: vec![],
            games_dir: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared axum state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    app_data_dir: PathBuf,
    games_path: PathBuf,
    config: AppConfig,
    app_handle: AppHandle,
}

impl AppState {
    fn read_games(&self) -> Vec<GameEntry> {
        let raw = fs::read_to_string(&self.games_path).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    }

    fn write_games(&self, games: &[GameEntry]) -> io::Result<()> {
        let json = serde_json::to_string_pretty(games).expect("serialize games");
        fs::write(&self.games_path, json)
    }

    /// Resolve the directory where a specific game's files live.
    fn game_dir(&self, id: &str) -> PathBuf {
        self.app_data_dir.join("games").join(id)
    }
}

type SharedState = Arc<Mutex<AppState>>;

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn load_games(state: tauri::State<SharedState>) -> Vec<GameEntry> {
    state.lock().unwrap().read_games()
}

#[tauri::command]
fn get_config(state: tauri::State<SharedState>) -> AppConfig {
    state.lock().unwrap().config.clone()
}

#[tauri::command]
fn launch_game(
    game_id: String,
    state: tauri::State<SharedState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let (exe_path, app_data_dir) = {
        let st = state.lock().unwrap();
        let games = st.read_games();
        let entry = games
            .iter()
            .find(|g| g.id == game_id)
            .ok_or_else(|| format!("game '{}' not found", game_id))?;

        if entry.executable_path.is_empty() {
            return Err(format!("game '{}' has no executable path set", game_id));
        }

        let exe = st.app_data_dir.join(&entry.executable_path);
        (exe, st.app_data_dir.clone())
    };

    // Minimize the launcher window while the game runs
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.minimize();
    }

    // Spawn the game process; watch for exit in a background task
    let mut child = std::process::Command::new(&exe_path)
        .current_dir(exe_path.parent().unwrap_or(&app_data_dir))
        .spawn()
        .map_err(|e| format!("failed to spawn game: {}", e))?;

    let app_handle_clone = app_handle.clone();
    let game_id_clone = game_id.clone();
    std::thread::spawn(move || {
        let _ = child.wait();
        if let Some(window) = app_handle_clone.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
        let _ = app_handle_clone.emit("gameExited", serde_json::json!({ "gameId": game_id_clone }));
    });

    Ok(())
}

#[tauri::command]
fn launch_mame(state: tauri::State<SharedState>, app_handle: AppHandle) -> Result<(), String> {
    let (mame_path, mame_args) = {
        let st = state.lock().unwrap();
        (st.config.mame_path.clone(), st.config.mame_args.clone())
    };

    // Minimize the launcher window while MAME runs
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.minimize();
    }

    let mut child = std::process::Command::new(&mame_path)
        .args(&mame_args)
        .spawn()
        .map_err(|e| format!("failed to spawn MAME: {}", e))?;

    let app_handle_clone = app_handle.clone();
    std::thread::spawn(move || {
        let _ = child.wait();
        if let Some(window) = app_handle_clone.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
        let _ = app_handle_clone.emit("mameExited", serde_json::json!({}));
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Axum handlers
// ---------------------------------------------------------------------------

async fn serve_admin() -> impl IntoResponse {
    Html(include_str!("admin.html"))
}

async fn get_games(State(state): State<SharedState>) -> impl IntoResponse {
    let games = state.lock().unwrap().read_games();
    Json(games)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateGamePayload {
    id: Option<String>,
    title: String,
    author: String,
    description: String,
    version: String,
    enabled: bool,
}

async fn upsert_game(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<CreateGamePayload>,
) -> impl IntoResponse {
    {
        let st = state.lock().unwrap();
        let expected_pin = st.config.admin_pin.clone();
        let provided_pin = headers
            .get("x-admin-pin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided_pin != expected_pin {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid PIN"}))).into_response();
        }
    }

    let id = {
        let st = state.lock().unwrap();
        let mut games = st.read_games();

        // Use provided id, or generate a new nanoid
        let id = payload.id.filter(|s| !s.is_empty()).unwrap_or_else(|| nanoid!(10));

        if let Some(existing) = games.iter_mut().find(|g| g.id == id) {
            // Preserve paths when editing metadata only
            existing.title = payload.title;
            existing.author = payload.author;
            existing.description = payload.description;
            existing.version = payload.version;
            existing.enabled = payload.enabled;
        } else {
            games.push(GameEntry {
                id: id.clone(),
                title: payload.title,
                author: payload.author,
                description: payload.description,
                thumbnail_path: String::new(),
                executable_path: String::new(),
                version: payload.version,
                enabled: payload.enabled,
            });
        }

        st.write_games(&games).expect("write games.json");
        id
    };

    // Emit event to renderer so the game grid refreshes
    {
        let st = state.lock().unwrap();
        let _ = st.app_handle.emit("gamesUpdated", ());
    }

    (StatusCode::OK, Json(serde_json::json!({"id": id}))).into_response()
}

async fn delete_game(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // PIN check
    {
        let st = state.lock().unwrap();
        let expected_pin = st.config.admin_pin.clone();
        let provided_pin = headers
            .get("x-admin-pin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided_pin != expected_pin {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid PIN"}))).into_response();
        }
    }

    {
        let st = state.lock().unwrap();
        let mut games = st.read_games();
        let before = games.len();
        games.retain(|g| g.id != id);

        if games.len() == before {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "game not found"}))).into_response();
        }

        st.write_games(&games).expect("write games.json");

        // Delete game folder (best-effort)
        let game_dir = st.game_dir(&id);
        if game_dir.exists() {
            let _ = fs::remove_dir_all(&game_dir);
        }

        let _ = st.app_handle.emit("gamesUpdated", ());
    }

    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

async fn upload_game(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // PIN check
    {
        let st = state.lock().unwrap();
        let expected_pin = st.config.admin_pin.clone();
        let provided_pin = headers
            .get("x-admin-pin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided_pin != expected_pin {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid PIN"}))).into_response();
        }
    }

    // Collect the ZIP bytes from the multipart field
    let mut zip_bytes: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("failed to read field: {}", e)})),
                )
                    .into_response();
            }
        };
        zip_bytes = Some(data.to_vec());
        break; // only expect one field
    }

    let zip_bytes = match zip_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "no file uploaded"})),
            )
                .into_response();
        }
    };

    // Determine the game directory and ensure it exists
    let game_dir = state.lock().unwrap().game_dir(&id);
    if let Err(e) = fs::create_dir_all(&game_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to create game dir: {}", e)})),
        )
            .into_response();
    }

    // Extract the ZIP
    let cursor = io::Cursor::new(zip_bytes);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid ZIP: {}", e)})),
            )
                .into_response();
        }
    };

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };

        // Sanitize the path — strip any leading path components that could escape the game dir
        let out_path = {
            let name = file.name();
            // Skip directory entries
            if name.ends_with('/') || name.ends_with('\\') {
                continue;
            }
            // Use only the final filename component to prevent path traversal
            let file_name = std::path::Path::new(name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(name);
            game_dir.join(file_name)
        };

        let mut out_file = match fs::File::create(&out_path) {
            Ok(f) => f,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("failed to write file: {}", e)})),
                )
                    .into_response();
            }
        };
        if let Err(e) = io::copy(&mut file, &mut out_file) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to extract file: {}", e)})),
            )
                .into_response();
        }
    }

    // Auto-detect exe from the extracted files
    let mut exe_rel: String = String::new();

    if let Ok(entries) = fs::read_dir(&game_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if exe_rel.is_empty() && name_str.to_lowercase().ends_with(".exe") {
                exe_rel = format!("games/{}/{}", id, name_str);
            }
        }
    }

    // Update (or create) the GameEntry with the detected exe path
    {
        let st = state.lock().unwrap();
        let mut games = st.read_games();

        if let Some(entry) = games.iter_mut().find(|g| g.id == id) {
            if !exe_rel.is_empty() {
                entry.executable_path = exe_rel;
            }
        } else {
            // Game entry doesn't exist yet — create a minimal one
            games.push(GameEntry {
                id: id.clone(),
                title: id.clone(),
                author: String::new(),
                description: String::new(),
                thumbnail_path: String::new(),
                executable_path: exe_rel,
                version: "1.0".to_string(),
                enabled: true,
            });
        }

        st.write_games(&games).expect("write games.json");
        let _ = st.app_handle.emit("gamesUpdated", ());
    }

    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

async fn upload_thumbnail(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // PIN check
    {
        let st = state.lock().unwrap();
        let expected_pin = st.config.admin_pin.clone();
        let provided_pin = headers
            .get("x-admin-pin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided_pin != expected_pin {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid PIN"}))).into_response();
        }
    }

    // Collect the image bytes and determine its extension from the filename
    let mut img_bytes: Option<Vec<u8>> = None;
    let mut img_ext = "png".to_string();
    while let Ok(Some(field)) = multipart.next_field().await {
        // Try to grab the original filename to preserve extension
        if let Some(fname) = field.file_name() {
            let lower = fname.to_lowercase();
            if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                img_ext = "jpg".to_string();
            } else if lower.ends_with(".gif") {
                img_ext = "gif".to_string();
            } else if lower.ends_with(".webp") {
                img_ext = "webp".to_string();
            }
        }
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("failed to read field: {}", e)})),
                )
                    .into_response();
            }
        };
        img_bytes = Some(data.to_vec());
        break;
    }

    let img_bytes = match img_bytes {
        Some(b) => b,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "no file uploaded"})),
            )
                .into_response();
        }
    };

    // Ensure game directory exists
    let game_dir = state.lock().unwrap().game_dir(&id);
    if let Err(e) = fs::create_dir_all(&game_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to create game dir: {}", e)})),
        )
            .into_response();
    }

    // Always save as thumbnail.<ext> for consistent serving
    let thumb_filename = format!("thumbnail.{}", img_ext);
    let thumb_path = game_dir.join(&thumb_filename);
    if let Err(e) = fs::write(&thumb_path, &img_bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to write thumbnail: {}", e)})),
        )
            .into_response();
    }

    let thumb_rel = format!("games/{}/{}", id, thumb_filename);

    // Update the GameEntry
    {
        let st = state.lock().unwrap();
        let mut games = st.read_games();

        if let Some(entry) = games.iter_mut().find(|g| g.id == id) {
            entry.thumbnail_path = thumb_rel.clone();
        } else {
            games.push(GameEntry {
                id: id.clone(),
                title: id.clone(),
                author: String::new(),
                description: String::new(),
                thumbnail_path: thumb_rel.clone(),
                executable_path: String::new(),
                version: "1.0".to_string(),
                enabled: true,
            });
        }

        st.write_games(&games).expect("write games.json");
        let _ = st.app_handle.emit("gamesUpdated", ());
    }

    (StatusCode::OK, Json(serde_json::json!({"ok": true, "thumbnailPath": thumb_rel}))).into_response()
}

async fn serve_game_file(
    State(state): State<SharedState>,
    Path((id, file)): Path<(String, String)>,
) -> Response {
    let game_dir = state.lock().unwrap().game_dir(&id);
    let file_path = game_dir.join(&file);

    // Prevent path traversal
    match file_path.canonicalize() {
        Ok(canonical) => {
            let game_dir_canonical = game_dir.canonicalize().unwrap_or(game_dir.clone());
            if !canonical.starts_with(&game_dir_canonical) {
                return StatusCode::FORBIDDEN.into_response();
            }
        }
        Err(_) => {
            return StatusCode::NOT_FOUND.into_response();
        }
    }

    match fs::read(&file_path) {
        Ok(bytes) => {
            let mime = mime_for_filename(&file);
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", mime)
                .body(Body::from(bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn mime_for_filename(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn ensure_config(config_path: &PathBuf) -> AppConfig {
    if config_path.exists() {
        let raw = fs::read_to_string(config_path).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        let config = AppConfig::default();
        let json = serde_json::to_string_pretty(&config).expect("serialize config");
        fs::write(config_path, json).expect("write config.json");
        config
    }
}

fn ensure_games(games_path: &PathBuf) {
    if !games_path.exists() {
        fs::write(games_path, "[]").expect("write games.json");
    }
}

fn spawn_admin_server(state: SharedState) {
    tauri::async_runtime::spawn(async move {
        let app = Router::new()
            .route("/", get(serve_admin))
            .route("/api/games", get(get_games))
            .route("/api/games", post(upsert_game))
            .route("/api/games/{id}", delete(delete_game))
            .route("/api/games/{id}/upload", post(upload_game))
            .route("/api/games/{id}/thumbnail", post(upload_thumbnail))
            .route("/games/{id}/{file}", get(serve_game_file))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8037")
            .await
            .expect("bind port 8037");

        axum::serve(listener, app).await.expect("axum serve");
    });
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("resolve app_data_dir");

            fs::create_dir_all(&app_data_dir).expect("create app_data_dir");

            // Ensure the games subfolder exists
            fs::create_dir_all(app_data_dir.join("games")).expect("create games dir");

            let config_path = app_data_dir.join("config.json");
            let games_path = app_data_dir.join("games.json");

            let config = ensure_config(&config_path);
            ensure_games(&games_path);

            let shared_state: SharedState = Arc::new(Mutex::new(AppState {
                app_data_dir,
                games_path,
                config,
                app_handle: app.handle().clone(),
            }));

            app.manage(shared_state.clone());

            spawn_admin_server(shared_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![load_games, get_config, launch_game, launch_mame])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
