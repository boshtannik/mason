//! User settings — the single source of the settings JSON key names.
//!
//! `serde` generates every key from a field name here, so neither the worker
//! nor the bridge writes a `["workdir"]` magic string anywhere.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_workdir")]
    pub workdir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            workdir: default_workdir(),
        }
    }
}

/// Default working directory for `opencode serve`.
pub fn default_workdir() -> String {
    "/home/defaultuser/mason".to_string()
}

fn settings_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/defaultuser".into());
    PathBuf::from(home)
        .join(".config/harbour-opencode")
        .join("settings.json")
}

impl Settings {
    /// Read settings from disk; missing keys/file fall back to defaults.
    pub fn load() -> Settings {
        let p = settings_path();
        std::fs::read_to_string(&p)
            .ok()
            .and_then(|txt| serde_json::from_str(&txt).ok())
            .unwrap_or_default()
    }

    /// Write settings to disk, creating the directory if needed.
    pub fn save(&self) {
        let p = settings_path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(txt) = serde_json::to_string(self) {
            let _ = std::fs::write(p, txt);
        }
    }

    /// JSON representation handed to QML.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}
