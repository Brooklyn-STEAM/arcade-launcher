use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use nanoid::nanoid;
use serde::Deserialize;
use std::{fs, io};
use tauri::Emitter;

use crate::models::{GameEntry, SharedState};

// ---------------------------------------------------------------------------
// Axum handlers
// ---------------------------------------------------------------------------

pub async fn serve_admin() -> impl IntoResponse {
    Html(include_str!("admin.html"))
}

pub async fn get_games(State(state): State<SharedState>) -> impl IntoResponse {
    let games = state.lock().unwrap().read_games();
    Json(games)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGamePayload {
    pub id: Option<String>,
    pub title: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub enabled: bool,
}

pub async fn upsert_game(
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

pub async fn delete_game(
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

pub async fn upload_game(
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

pub async fn upload_thumbnail(
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

pub async fn serve_game_file(
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
