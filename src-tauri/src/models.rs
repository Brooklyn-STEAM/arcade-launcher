use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::AppHandle;

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
pub struct AppState {
    pub app_data_dir: PathBuf,
    pub games_path: PathBuf,
    pub config: AppConfig,
    pub app_handle: AppHandle,
}

impl AppState {
    pub fn read_games(&self) -> Vec<GameEntry> {
        let raw = fs::read_to_string(&self.games_path).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn write_games(&self, games: &[GameEntry]) -> io::Result<()> {
        let json = serde_json::to_string_pretty(games).expect("serialize games");
        fs::write(&self.games_path, json)
    }

    /// Resolve the directory where a specific game's files live.
    pub fn game_dir(&self, id: &str) -> PathBuf {
        self.app_data_dir.join("games").join(id)
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
