use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub hotkey: String,
    pub model_size: String,
    pub language: String,
    pub output_mode: String,
    pub filler_removal: bool,
    pub sound_effects: bool,
    pub auto_start: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+Space".to_string(),
            model_size: "base".to_string(),
            language: "auto".to_string(),
            output_mode: "clipboard_paste".to_string(),
            filler_removal: true,
            sound_effects: true,
            auto_start: false,
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

    /// Load settings from a specific path. Returns defaults if the file doesn't exist or can't be parsed.
    pub fn load_from(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }

        let contents = match fs::read_to_string(path) {
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

    /// Load settings from disk. Returns defaults if the file doesn't exist or can't be parsed.
    pub fn load() -> Self {
        let path = match Self::file_path() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Could not determine settings path: {}", e);
                return Self::default();
            }
        };

        Self::load_from(&path)
    }

    /// Save settings to a specific path. Creates parent directories if needed.
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(path, json)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;

        Ok(())
    }

    /// Save settings to disk. Creates the Scribe directory if it doesn't exist.
    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path()?;
        self.save_to(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Helper: get a settings.json path inside a temp directory
    fn temp_settings_path(dir: &TempDir) -> PathBuf {
        dir.path().join("Scribe").join("settings.json")
    }

    // ================================================================
    // FIRST LAUNCH SCENARIOS
    // ================================================================

    #[test]
    fn user_launches_app_first_time_gets_sensible_defaults() {
        // UX: User installs Scribe and launches it. No settings file exists yet.
        // They expect the app to "just work" with reasonable defaults:
        // auto language detection, clipboard paste output, and the default hotkey.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        let settings = Settings::load_from(&path);

        assert_eq!(settings.language, "auto", "Language should default to auto-detect");
        assert_eq!(settings.output_mode, "clipboard_paste", "Output should default to clipboard paste");
        assert_eq!(settings.hotkey, "Ctrl+Shift+Space", "Hotkey should have a sensible default");
        assert_eq!(settings.model_size, "base", "Model should default to base for fast startup");
    }

    #[test]
    fn user_launches_app_with_corrupt_settings_file_gets_defaults_not_crash() {
        // UX: Something went wrong (disk error, manual edit gone bad). The settings
        // file contains garbage. The user expects the app to still launch normally,
        // falling back to defaults rather than showing a cryptic error.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "this is not valid json {{{").unwrap();

        let settings = Settings::load_from(&path);

        assert_eq!(settings, Settings::default(),
            "Corrupt settings should fall back to defaults, not crash");
    }

    #[test]
    fn user_launches_app_with_partial_settings_preserves_existing_fills_missing() {
        // UX: User upgraded from an older version that only had hotkey and language.
        // The new version added output_mode and model_size. The user expects their
        // existing preferences to be preserved, with new fields getting defaults.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Truly partial JSON — only 2 of 4 fields (simulates older app version)
        let partial_json = r#"{
            "hotkey": "Alt+R",
            "language": "ja"
        }"#;
        fs::write(&path, partial_json).unwrap();

        let settings = Settings::load_from(&path);

        // Existing fields preserved
        assert_eq!(settings.hotkey, "Alt+R", "Existing hotkey should be preserved");
        assert_eq!(settings.language, "ja", "Existing language should be preserved");
        // Missing fields filled with defaults
        assert_eq!(settings.model_size, "base", "Missing model_size should get default");
        assert_eq!(settings.output_mode, "clipboard_paste", "Missing output_mode should get default");
    }

    // ================================================================
    // PERSISTENCE SCENARIOS
    // ================================================================

    #[test]
    fn user_changes_language_to_japanese_persists_across_restart() {
        // UX: User opens Settings, selects Japanese, closes app. Next day they
        // reopen Scribe. They expect it to still be set to Japanese.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        let mut settings = Settings::default();
        settings.language = "ja".to_string();
        settings.save_to(&path).unwrap();

        // Simulate app restart: load fresh from disk
        let reloaded = Settings::load_from(&path);
        assert_eq!(reloaded.language, "ja",
            "Language setting should survive app restart");
    }

    #[test]
    fn user_changes_output_mode_to_direct_type_persists_across_restart() {
        // UX: User switches to direct typing mode because their target app
        // doesn't support paste. After restarting, this preference must stick.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        let mut settings = Settings::default();
        settings.output_mode = "direct_type".to_string();
        settings.save_to(&path).unwrap();

        let reloaded = Settings::load_from(&path);
        assert_eq!(reloaded.output_mode, "direct_type",
            "Output mode should survive app restart");
    }

    #[test]
    fn user_changes_hotkey_persists_across_restart() {
        // UX: User remaps the hotkey to avoid conflicts with another app.
        // After restart, their custom hotkey must still be there.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        let mut settings = Settings::default();
        settings.hotkey = "Ctrl+Alt+V".to_string();
        settings.save_to(&path).unwrap();

        let reloaded = Settings::load_from(&path);
        assert_eq!(reloaded.hotkey, "Ctrl+Alt+V",
            "Custom hotkey should survive app restart");
    }

    #[test]
    fn all_settings_survive_full_round_trip() {
        // UX: User customizes every single setting. After restart, ALL of them
        // must come back exactly as they were. Not just one field -- everything.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        let original = Settings {
            hotkey: "F9".to_string(),
            model_size: "large-v3-turbo-q5_0".to_string(),
            language: "de".to_string(),
            output_mode: "clipboard_only".to_string(),
            filler_removal: false,
            sound_effects: false,
            auto_start: true,
        };

        original.save_to(&path).unwrap();

        // Simulate fresh app instance loading from same path
        let restored = Settings::load_from(&path);
        assert_eq!(original, restored,
            "Every field must survive the save-load round trip");
    }

    // ================================================================
    // EDGE CASE SCENARIOS
    // ================================================================

    #[test]
    fn settings_file_deleted_while_running_next_save_recreates_it() {
        // UX: User (or antivirus) deletes settings.json while app is running.
        // When the user changes a setting, the app should recreate the file
        // rather than error out.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        let settings = Settings::default();
        settings.save_to(&path).unwrap();
        assert!(path.exists(), "Settings file should exist after first save");

        // Simulate deletion
        fs::remove_file(&path).unwrap();
        assert!(!path.exists(), "Settings file should be gone after deletion");

        // User changes a setting -- save should recreate the file
        let mut modified = settings.clone();
        modified.language = "fr".to_string();
        modified.save_to(&path).unwrap();
        assert!(path.exists(), "Settings file should be recreated on save");

        let reloaded = Settings::load_from(&path);
        assert_eq!(reloaded.language, "fr",
            "Recreated settings should have the new value");
    }

    #[test]
    fn settings_directory_doesnt_exist_save_creates_entire_tree() {
        // UX: Fresh install or user cleaned up %APPDATA%. The Scribe directory
        // doesn't exist. Saving settings should create the full path.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("deeply").join("nested").join("dir").join("settings.json");

        let settings = Settings::default();
        settings.save_to(&path).unwrap();

        assert!(path.exists(),
            "Save should create the entire directory tree if needed");

        let reloaded = Settings::load_from(&path);
        assert_eq!(reloaded, Settings::default(),
            "Settings saved to a newly-created directory should load back correctly");
    }

    #[test]
    fn settings_with_unknown_extra_fields_loads_without_error() {
        // UX: Future version of Scribe adds new settings fields. User downgrades
        // back to this version. The settings file has fields this version doesn't
        // know about. It must not crash -- just ignore the unknown fields.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let json_with_extras = r#"{
            "hotkey": "Ctrl+Shift+Space",
            "model_size": "base",
            "language": "auto",
            "output_mode": "clipboard_paste",
            "future_feature": true,
            "theme": "dark",
            "ai_enhancement_level": 42
        }"#;
        fs::write(&path, json_with_extras).unwrap();

        let settings = Settings::load_from(&path);

        // Known fields should load correctly
        assert_eq!(settings.hotkey, "Ctrl+Shift+Space");
        assert_eq!(settings.language, "auto");
        // The fact that we got here without panicking proves forward compatibility
    }

    #[test]
    fn settings_file_is_saved_as_human_readable_json() {
        // UX: Power user opens settings.json in a text editor to manually tweak
        // values. The file should be pretty-printed, not a single compressed line.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        Settings::default().save_to(&path).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains('\n'),
            "Settings file should be pretty-printed for manual editing");
        assert!(contents.contains("  "),
            "Settings file should use indentation");
    }

    #[test]
    fn empty_settings_file_falls_back_to_defaults() {
        // UX: Settings file exists but is empty (0 bytes). Could happen from
        // a crash during write. App should not crash, just use defaults.
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "").unwrap();

        let settings = Settings::load_from(&path);
        assert_eq!(settings, Settings::default(),
            "Empty settings file should fall back to defaults");
    }

    // ================================================================
    // FILLER REMOVAL SETTINGS
    // ================================================================

    #[test]
    fn filler_removal_defaults_to_true_and_persists() {
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        let settings = Settings::load_from(&path);
        assert_eq!(settings.filler_removal, true, "Should default to ON");

        let mut settings = Settings::default();
        settings.filler_removal = false;
        settings.save_to(&path).unwrap();

        let reloaded = Settings::load_from(&path);
        assert_eq!(reloaded.filler_removal, false, "Disabled should persist");
    }

    #[test]
    fn old_settings_without_filler_removal_get_default_true() {
        let dir = TempDir::new().unwrap();
        let path = temp_settings_path(&dir);

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let old_json = r#"{
            "hotkey": "Ctrl+Shift+Space",
            "model_size": "base",
            "language": "en",
            "output_mode": "clipboard_paste"
        }"#;
        fs::write(&path, old_json).unwrap();

        let settings = Settings::load_from(&path);
        assert_eq!(settings.filler_removal, true, "Missing field should default to true");
        assert_eq!(settings.language, "en", "Existing fields preserved");
    }
}
