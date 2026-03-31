use axum::{
    routing::{delete, get, post},
    Router,
};
use std::{fs, sync::{Arc, Mutex}};
use tauri::Manager;
use tower_http::cors::CorsLayer;

pub mod admin_api;
pub mod commands;
pub mod models;

use admin_api::{
    delete_game, get_games, serve_admin, serve_game_file, upload_game, upload_thumbnail,
    upsert_game,
};
use commands::{
    check_for_update, get_config, get_local_ip, install_update, launch_game, launch_mame,
    load_games,
};
use models::{AppConfig, AppState, SharedState};

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn ensure_config(config_path: &std::path::PathBuf) -> AppConfig {
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

fn ensure_games(games_path: &std::path::PathBuf) {
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
        .plugin(tauri_plugin_updater::Builder::new().build())
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

            // Go fullscreen in production builds; stay windowed during dev
            if !cfg!(debug_assertions) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_fullscreen(true);
                    let _ = window.set_decorations(false);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_games,
            get_config,
            get_local_ip,
            launch_game,
            launch_mame,
            check_for_update,
            install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
