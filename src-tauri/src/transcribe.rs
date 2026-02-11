use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
    pub language: String,
    pub duration: f32,
}

pub struct PythonService {
    process: Option<Child>,
    base_url: String,
}

impl PythonService {
    pub fn new() -> Self {
        Self {
            process: None,
            base_url: "http://127.0.0.1:8765".to_string(),
        }
    }

    pub async fn start(&mut self) -> Result<(), String> {
        // Path to Python script (in development, use direct path)
        // In production, this would be the bundled sidecar executable
        let python_script = std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join("python")
            .join("whisper_service.py");

        // Try to find Python
        let python_cmd = if cfg!(windows) {
            "python"
        } else {
            "python3"
        };

        println!("Starting Python service: {:?}", python_script);

        let child = Command::new(python_cmd)
            .arg(&python_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start Python service: {}", e))?;

        self.process = Some(child);

        // Wait for service to be ready
        for i in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if self.health_check().await.is_ok() {
                println!("Python service ready after {}ms", i * 500);
                return Ok(());
            }
        }

        Err("Python service failed to start within 10 seconds".to_string())
    }

    pub async fn health_check(&self) -> Result<bool, String> {
        let client = reqwest::Client::new();
        let url = format!("{}/health", self.base_url);

        match client
            .get(&url)
            .timeout(Duration::from_millis(500))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => Ok(true),
            Ok(_) => Err("Health check returned non-200 status".to_string()),
            Err(e) => Err(format!("Health check failed: {}", e)),
        }
    }

    pub async fn transcribe(&self, audio_path: PathBuf) -> Result<TranscriptionResponse, String> {
        let client = reqwest::Client::new();
        let url = format!("{}/transcribe", self.base_url);

        // Read audio file
        let file_bytes = tokio::fs::read(&audio_path)
            .await
            .map_err(|e| format!("Failed to read audio file: {}", e))?;

        let file_name = audio_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("recording.wav");

        // Create multipart form
        let part = multipart::Part::bytes(file_bytes)
            .file_name(file_name.to_string())
            .mime_str("audio/wav")
            .map_err(|e| e.to_string())?;

        let form = multipart::Form::new().part("audio", part);

        // Send request
        let response = client
            .post(&url)
            .multipart(form)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Transcription request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Transcription failed: {}", error_text));
        }

        let result: TranscriptionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(result)
    }

    pub fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // Try graceful shutdown via stdin
            if let Some(stdin) = process.stdin.as_mut() {
                use std::io::Write;
                let _ = writeln!(stdin, "SHUTDOWN");
            }

            // Wait for exit
            let _ = process.wait();
        }
    }
}

impl Drop for PythonService {
    fn drop(&mut self) {
        self.stop();
    }
}
