use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::models::{AppConfig, GameEntry, SharedState};

// ---------------------------------------------------------------------------
// Updater
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub body: Option<String>,
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(UpdateInfo {
            version: update.version.clone(),
            body: update.body.clone(),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no update available".to_string())?;

    update
        .download_and_install(
            |_chunk, _total| {},
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    app.restart();
}

// ---------------------------------------------------------------------------
// Game commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn load_games(state: tauri::State<SharedState>) -> Vec<GameEntry> {
    state.lock().unwrap().read_games()
}

#[tauri::command]
pub fn get_config(state: tauri::State<SharedState>) -> AppConfig {
    state.lock().unwrap().config.clone()
}

#[tauri::command]
pub fn get_local_ip() -> String {
    // Open a UDP socket aimed at an external address — no data is sent.
    // The OS picks the appropriate local interface, letting us read our LAN IP.
    use std::net::UdpSocket;
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}

#[tauri::command]
pub fn launch_game(
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
pub fn launch_mame(state: tauri::State<SharedState>, app_handle: AppHandle) -> Result<(), String> {
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
