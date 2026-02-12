use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub model_size: String,
    pub language: String,
    pub output_mode: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+Space".to_string(),
            model_size: "base".to_string(),
            language: "auto".to_string(),
            output_mode: "clipboard_paste".to_string(),
        }
    }
}

impl Settings {
    /// Returns the path to the settings file: %APPDATA%/Scribe/settings.json
    fn file_path() -> Result<PathBuf, String> {
        let appdata = std::env::var("APPDATA")
            .map_err(|_| "APPDATA environment variable not set".to_string())?;
        Ok(PathBuf::from(appdata).join("Scribe").join("settings.json"))
    }

    /// Load settings from disk. Returns defaults if the file doesn't exist or can't be parsed.
    pub fn load() -> Self {
        let path = match Self::file_path() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Could not determine settings path: {}", e);
                return Self::default();
            }
        };

        if !path.exists() {
            return Self::default();
        }

        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to read settings file: {}", e);
                return Self::default();
            }
        };

        match serde_json::from_str(&contents) {
            Ok(settings) => settings,
            Err(e) => {
                eprintln!("Failed to parse settings file: {}", e);
                Self::default()
            }
        }
    }

    /// Save settings to disk. Creates the Scribe directory if it doesn't exist.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&path, json)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;

        Ok(())
    }
}
