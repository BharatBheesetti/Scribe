use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tauri::{AppHandle, Emitter};

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

struct ModelDef {
    name: &'static str,
    filename: &'static str,
    url: &'static str,
    size_mb: u64,
    description: &'static str,
}

const MODELS: &[ModelDef] = &[
    ModelDef {
        name: "base.en",
        filename: "ggml-base.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        size_mb: 148,
        description: "Fast, English-only",
    },
    ModelDef {
        name: "small.en",
        filename: "ggml-small.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        size_mb: 488,
        description: "Better accuracy, English-only",
    },
    ModelDef {
        name: "large-v3-turbo-q5_0",
        filename: "ggml-large-v3-turbo-q5_0.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
        size_mb: 574,
        description: "Best accuracy, multilingual",
    },
];

pub const DEFAULT_MODEL: &str = "base.en";

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub name: String,
    pub filename: String,
    pub size_mb: u64,
    pub description: String,
    pub downloaded: bool,
    pub active: bool,
}

pub fn models_dir() -> Result<PathBuf, String> {
    let appdata =
        std::env::var("APPDATA").map_err(|_| "APPDATA environment variable not set".to_string())?;
    Ok(PathBuf::from(appdata).join("Scribe").join("models"))
}

pub fn model_path(filename: &str) -> Result<PathBuf, String> {
    Ok(models_dir()?.join(filename))
}

pub fn is_downloaded(filename: &str) -> bool {
    model_path(filename).map(|p| p.exists()).unwrap_or(false)
}

/// Check if a model file exists under a specific base directory.
fn is_downloaded_in(base_dir: &Path, filename: &str) -> bool {
    base_dir.join(filename).exists()
}

pub fn default_model_path() -> Result<Option<PathBuf>, String> {
    let def = MODELS
        .iter()
        .find(|m| m.name == DEFAULT_MODEL)
        .ok_or("Default model not found in registry")?;
    let path = model_path(def.filename)?;
    if path.exists() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

/// Return the file path for a model by name (regardless of whether it exists).
pub fn path_for_model(name: &str) -> Result<PathBuf, String> {
    let model = MODELS
        .iter()
        .find(|m| m.name == name)
        .ok_or_else(|| format!("Unknown model: {}", name))?;
    model_path(model.filename)
}

/// Return the file path for a model by name within a specific base directory.
#[cfg(test)]
fn path_for_model_in(base_dir: &Path, name: &str) -> Result<PathBuf, String> {
    let model = MODELS
        .iter()
        .find(|m| m.name == name)
        .ok_or_else(|| format!("Unknown model: {}", name))?;
    Ok(base_dir.join(model.filename))
}

/// List all known models with their download and active status.
pub fn list_models(active_model: &str) -> Vec<ModelStatus> {
    list_models_in_dir(active_model, None)
}

/// List all known models, checking download status against a specific directory.
/// If base_dir is None, uses the default models_dir().
fn list_models_in_dir(active_model: &str, base_dir: Option<&Path>) -> Vec<ModelStatus> {
    MODELS
        .iter()
        .map(|m| {
            let downloaded = match base_dir {
                Some(dir) => is_downloaded_in(dir, m.filename),
                None => is_downloaded(m.filename),
            };
            ModelStatus {
                name: m.name.to_string(),
                filename: m.filename.to_string(),
                size_mb: m.size_mb,
                description: m.description.to_string(),
                downloaded,
                active: m.name == active_model,
            }
        })
        .collect()
}

/// Download a model from HuggingFace, emitting progress events.
pub async fn download_model(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let model = MODELS
        .iter()
        .find(|m| m.name == name)
        .ok_or_else(|| format!("Unknown model: {}", name))?;

    let dir = models_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create models directory: {}", e))?;

    let dest = dir.join(model.filename);

    println!(
        "Downloading model '{}' from {} to {:?}",
        name, model.url, dest
    );

    let response = HTTP_CLIENT
        .get(model.url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with HTTP {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_progress: u32 = 0;

    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|e| format!("Failed to create model file: {}", e))?;

    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("File write error: {}", e))?;

        downloaded += chunk.len() as u64;

        let progress = if total_size > 0 {
            (downloaded as f64 / total_size as f64 * 100.0) as u32
        } else {
            0
        };

        // Emit progress every 1% to avoid flooding the event bus
        if progress > last_progress {
            last_progress = progress;
            let _ = app.emit(
                "model-download-progress",
                serde_json::json!({
                    "model": name,
                    "progress": progress,
                    "downloaded_mb": downloaded / 1_000_000,
                    "total_mb": total_size / 1_000_000,
                }),
            );
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("File flush error: {}", e))?;

    println!("Download complete: {:?} ({} MB)", dest, downloaded / 1_000_000);

    let _ = app.emit(
        "model-download-complete",
        serde_json::json!({ "model": name }),
    );

    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ================================================================
    // MODEL REGISTRY SCENARIOS
    // ================================================================

    #[test]
    fn user_sees_all_available_models_in_settings_ui() {
        // UX: User opens Settings > Models tab. They see a list of available
        // models with names, sizes, and descriptions to help them choose.
        let dir = TempDir::new().unwrap();
        let models = list_models_in_dir("base.en", Some(dir.path()));

        assert_eq!(models.len(), 3, "User should see exactly 3 model options");

        // Verify each model has the info users need to make a choice
        let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"base.en"), "Should include base.en model");
        assert!(names.contains(&"small.en"), "Should include small.en model");
        assert!(names.contains(&"large-v3-turbo-q5_0"), "Should include large model");

        // Every model must have a description so users know what they're getting
        for model in &models {
            assert!(!model.description.is_empty(),
                "Model '{}' needs a description for the user", model.name);
            assert!(model.size_mb > 0,
                "Model '{}' should show its download size", model.name);
            assert!(!model.filename.is_empty(),
                "Model '{}' must have a filename", model.name);
        }
    }

    #[test]
    fn user_sees_which_model_is_currently_active() {
        // UX: User opens model list. The currently active model should be
        // highlighted so they know which one is in use.
        let dir = TempDir::new().unwrap();
        let models = list_models_in_dir("small.en", Some(dir.path()));

        let active_count = models.iter().filter(|m| m.active).count();
        assert_eq!(active_count, 1, "Exactly one model should be marked active");

        let active = models.iter().find(|m| m.active).unwrap();
        assert_eq!(active.name, "small.en", "The active model should match what was set");
    }

    #[test]
    fn user_sees_correct_download_status_for_downloaded_model() {
        // UX: User downloaded base.en model earlier. When they open Settings,
        // it should show as "Downloaded" (green checkmark), not "Download" button.
        let dir = TempDir::new().unwrap();

        // Simulate a previously downloaded model file
        let model_file = dir.path().join("ggml-base.en.bin");
        fs::write(&model_file, "fake model data for testing").unwrap();

        let models = list_models_in_dir("base.en", Some(dir.path()));
        let base = models.iter().find(|m| m.name == "base.en").unwrap();

        assert!(base.downloaded,
            "Model with file on disk should show as downloaded");
    }

    #[test]
    fn user_sees_correct_download_status_for_missing_model() {
        // UX: User hasn't downloaded small.en yet. It should show a "Download"
        // button, not pretend it's available.
        let dir = TempDir::new().unwrap();
        // Empty directory -- no model files exist

        let models = list_models_in_dir("base.en", Some(dir.path()));
        let small = models.iter().find(|m| m.name == "small.en").unwrap();

        assert!(!small.downloaded,
            "Model without file on disk should show as not downloaded");
    }

    #[test]
    fn model_download_detection_distinguishes_between_models() {
        // UX: User downloaded base.en but not small.en. The UI must show the
        // correct status for each -- not mark all as downloaded just because
        // one exists.
        let dir = TempDir::new().unwrap();

        // Only create base.en file
        fs::write(dir.path().join("ggml-base.en.bin"), "data").unwrap();

        let models = list_models_in_dir("base.en", Some(dir.path()));

        let base = models.iter().find(|m| m.name == "base.en").unwrap();
        let small = models.iter().find(|m| m.name == "small.en").unwrap();
        let large = models.iter().find(|m| m.name == "large-v3-turbo-q5_0").unwrap();

        assert!(base.downloaded, "base.en was downloaded");
        assert!(!small.downloaded, "small.en was NOT downloaded");
        assert!(!large.downloaded, "large model was NOT downloaded");
    }

    // ================================================================
    // MODEL PATH RESOLUTION
    // ================================================================

    #[test]
    fn model_path_resolves_to_correct_appdata_location() {
        // UX: When user clicks "Download", the model needs to go to the right
        // place so it can be found on next launch. The path must be under
        // %APPDATA%/Scribe/models/ for proper Windows integration.
        let dir = models_dir();

        // APPDATA should be set on all Windows machines
        assert!(dir.is_ok(), "models_dir should resolve on Windows");

        let dir = dir.unwrap();
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.contains("Scribe"),
            "Model directory should be under Scribe app folder");
        assert!(dir_str.contains("models"),
            "Model directory should be in a 'models' subfolder");
    }

    #[test]
    fn path_for_known_model_returns_correct_filename() {
        // UX: When the inference engine needs to load a model, it looks up the
        // path by model name. The filename must match what HuggingFace distributes.
        let dir = TempDir::new().unwrap();

        let path = path_for_model_in(dir.path(), "base.en").unwrap();
        assert!(path.to_string_lossy().ends_with("ggml-base.en.bin"),
            "base.en should resolve to ggml-base.en.bin");

        let path = path_for_model_in(dir.path(), "small.en").unwrap();
        assert!(path.to_string_lossy().ends_with("ggml-small.en.bin"),
            "small.en should resolve to ggml-small.en.bin");

        let path = path_for_model_in(dir.path(), "large-v3-turbo-q5_0").unwrap();
        assert!(path.to_string_lossy().ends_with("ggml-large-v3-turbo-q5_0.bin"),
            "large model should resolve to correct bin filename");
    }

    #[test]
    fn path_for_unknown_model_returns_clear_error() {
        // UX: If somehow an invalid model name reaches the backend (UI bug,
        // config corruption), the error message should be clear for debugging.
        let dir = TempDir::new().unwrap();
        let result = path_for_model_in(dir.path(), "nonexistent-model");

        assert!(result.is_err(), "Unknown model should return an error");
        let err = result.unwrap_err();
        assert!(err.contains("Unknown model"),
            "Error should clearly state the model is unknown, got: {}", err);
    }

    #[test]
    fn model_sizes_are_reasonable_for_download_progress_display() {
        // UX: The download progress UI shows "148 MB" or "1.0 GB" to set user
        // expectations. These values must be approximately correct so users can
        // decide if they want to wait.
        let dir = TempDir::new().unwrap();
        let models = list_models_in_dir("base.en", Some(dir.path()));

        let base = models.iter().find(|m| m.name == "base.en").unwrap();
        assert!(base.size_mb > 100 && base.size_mb < 300,
            "base.en should be ~148MB, got {}MB", base.size_mb);

        let small = models.iter().find(|m| m.name == "small.en").unwrap();
        assert!(small.size_mb > 300 && small.size_mb < 700,
            "small.en should be ~488MB, got {}MB", small.size_mb);

        let large = models.iter().find(|m| m.name == "large-v3-turbo-q5_0").unwrap();
        assert!(large.size_mb > 400 && large.size_mb < 700,
            "large model should be ~574MB, got {}MB", large.size_mb);
    }

    #[test]
    fn each_model_has_a_unique_filename() {
        // UX: If two models shared a filename, downloading one would overwrite
        // the other. Each model must have its own unique file.
        let dir = TempDir::new().unwrap();
        let models = list_models_in_dir("base.en", Some(dir.path()));
        let filenames: Vec<&str> = models.iter().map(|m| m.filename.as_str()).collect();

        let mut unique = filenames.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(filenames.len(), unique.len(),
            "Every model must have a unique filename to prevent overwrites");
    }

    #[test]
    fn each_model_has_a_unique_name() {
        // UX: Model names are used as identifiers in the UI and settings.
        // Duplicate names would cause the wrong model to be selected.
        let dir = TempDir::new().unwrap();
        let models = list_models_in_dir("base.en", Some(dir.path()));
        let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();

        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len(),
            "Every model must have a unique name for correct selection");
    }
}
