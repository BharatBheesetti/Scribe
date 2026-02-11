# MAATA - Technical Architecture
## Based on SPEC.md v1.0

---

## 1. SYSTEM ARCHITECTURE

```
┌─────────────────────────────────────────────────────────────────┐
│                        USER INTERACTION                          │
│  Global Hotkey (Ctrl+Shift+Space) + System Tray + Notifications │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    TAURI MAIN PROCESS (Rust)                     │
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   hotkey.rs  │  │   tray.rs    │  │   audio.rs   │          │
│  │              │  │              │  │              │          │
│  │ - Register   │  │ - Icon mgmt  │  │ - CPAL       │          │
│  │   Ctrl+Shift │  │ - Menu       │  │ - 16kHz mono │          │
│  │   +Space     │  │ - 3 states   │  │ - WAV write  │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                  │                  │                  │
│         └──────────────────┼──────────────────┘                  │
│                            │                                     │
│                            ▼                                     │
│                    ┌──────────────┐                              │
│                    │   main.rs    │                              │
│                    │              │                              │
│                    │ - App state  │                              │
│                    │ - Event loop │                              │
│                    │ - Orchestr.  │                              │
│                    └──────┬───────┘                              │
│                           │                                      │
│         ┌─────────────────┼─────────────────┐                   │
│         │                 │                 │                   │
│         ▼                 ▼                 ▼                   │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │transcribe.rs│  │  typing.rs   │  │Settings (TBD)│           │
│  │             │  │              │  │              │           │
│  │- HTTP client│  │- enigo       │  │- JSON store  │           │
│  │- Multipart  │  │- Clipboard   │  │- Persist     │           │
│  └─────┬───────┘  └──────────────┘  └──────────────┘           │
└────────┼────────────────────────────────────────────────────────┘
         │ HTTP POST
         │ localhost:8765/transcribe
         ▼
┌─────────────────────────────────────────────────────────────────┐
│               PYTHON SUBPROCESS (FastAPI Server)                 │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   whisper_service.py                      │   │
│  │                                                            │   │
│  │  FastAPI App:                                             │   │
│  │  - POST /transcribe (multipart WAV file)                 │   │
│  │  - GET /health                                            │   │
│  │                                                            │   │
│  │  WhisperModel:                                            │   │
│  │  - Loaded once on startup                                │   │
│  │  - Kept in memory (GPU/CPU)                              │   │
│  │  - medium model (1.5GB)                                  │   │
│  │                                                            │   │
│  │  Lifecycle:                                               │   │
│  │  - Started by Tauri on app launch                        │   │
│  │  - Listens on 127.0.0.1:8765                             │   │
│  │  - Graceful shutdown via stdin "SHUTDOWN\n"              │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. MODULE BREAKDOWN

### 2.1 main.rs

**Purpose:** Application entry point and orchestration

**Key Responsibilities:**
- Initialize Tauri app
- Setup global hotkey handlers
- Setup system tray
- Manage application state
- Start Python subprocess
- Coordinate event flow between modules

**State:**
```rust
struct AppState {
    recorder: Arc<Mutex<AudioRecorder>>,
    python_service: Arc<Mutex<PythonService>>,
    is_recording: Arc<Mutex<bool>>,
    settings: Arc<Mutex<Settings>>,
}
```

**Event Loop:**
1. Listen for `hotkey-pressed` → Call audio.start_recording()
2. Listen for `hotkey-released` → Call audio.stop_recording() → transcribe.send_audio() → typing.auto_type()
3. Listen for `escape-pressed` → Cancel recording, cleanup

**Dependencies:**
- tauri
- All internal modules (audio, hotkey, tray, transcribe, typing)
- tokio (async runtime)

---

### 2.2 audio.rs

**Purpose:** Audio capture using CPAL

**Public API:**
```rust
pub struct AudioRecorder {
    sample_rate: u32,
    channels: u16,
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    start_time: Option<Instant>,
}

impl AudioRecorder {
    pub fn new() -> Self;
    pub fn start_recording(&mut self) -> Result<(), String>;
    pub fn stop_recording(&mut self) -> Result<PathBuf, String>;
    pub fn cancel_recording(&mut self);
}
```

**Implementation Details:**
- **Sample Rate:** 16kHz (Whisper native)
- **Channels:** Mono
- **Buffer:** Arc<Mutex<Vec<f32>>> for thread-safe sample collection
- **Stream:** Created once, reused for all recordings
- **Max Duration:** 60 seconds (enforced)
- **Output:** WAV file in temp directory

**Thread Safety:**
- Audio callback runs on high-priority CPAL thread
- Shared samples buffer protected by Mutex
- Keep callback fast (just push samples, no processing)

**Error Handling:**
- Device not found → Return error with user-friendly message
- Permission denied → Return specific error for UI to show instructions
- Buffer overflow (>60s) → Auto-stop and return error

---

### 2.3 hotkey.rs

**Purpose:** Global hotkey registration and handling

**Public API:**
```rust
pub fn setup_hotkeys(app: &AppHandle) -> Result<(), Box<dyn Error>>;
pub fn register_recording_hotkey(app: &AppHandle, shortcut: &str) -> Result<(), Box<dyn Error>>;
```

**Implementation:**
- Use `tauri-plugin-global-shortcut`
- Register Ctrl+Shift+Space on app startup
- Register Escape key for cancellation
- Emit events to main process:
  - `hotkey-pressed`
  - `hotkey-released`
  - `escape-pressed`

**Configuration:**
- Hotkey customization stored in settings.json
- Default: Ctrl+Shift+Space
- Validation: Prevent conflicting/invalid combos

---

### 2.4 tray.rs

**Purpose:** System tray icon and menu management

**Public API:**
```rust
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn Error>>;
pub fn update_tray_icon(app: &AppHandle, state: TrayState) -> Result<(), Box<dyn Error>>;

pub enum TrayState {
    Idle,       // White icon
    Recording,  // Red icon
    Processing, // Yellow icon
}
```

**Menu Items:**
- "Start Recording (Ctrl+Shift+Space)" → Trigger recording
- "Settings" → Open settings window
- "Quit" → Graceful shutdown

**Icon Management:**
- Load 3 icon variants from resources/
- Update icon based on app state
- Immediate visual feedback (<50ms)

---

### 2.5 transcribe.rs

**Purpose:** Communication with Python service

**Public API:**
```rust
pub struct PythonService {
    process: Option<Child>,
    base_url: String,
}

impl PythonService {
    pub fn new() -> Self;
    pub async fn start(&mut self) -> Result<(), String>;
    pub async fn stop(&mut self);
    pub async fn transcribe(&self, audio_path: PathBuf) -> Result<TranscriptionResponse, String>;
    pub async fn health_check(&self) -> Result<bool, String>;
}

#[derive(Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
    pub language: String,
    pub duration: f32,
}
```

**Process Lifecycle:**
1. **Start:** Spawn Python executable (bundled via sidecar)
2. **Health Check:** Poll /health endpoint until ready (max 10s)
3. **Transcribe:** POST /transcribe with multipart WAV file
4. **Shutdown:** Send "SHUTDOWN\n" via stdin, wait for graceful exit

**HTTP Client:**
- Use reqwest with timeout (60s per request)
- Multipart file upload
- Retry logic: 3 attempts with exponential backoff
- Connection pooling for reuse

**Error Handling:**
- Service not started → Return error
- Timeout → Return error with retry suggestion
- Network error → Check if process crashed, restart if needed
- Parse error → Log response, return generic error

---

### 2.6 typing.rs

**Purpose:** Auto-type text into active application

**Public API:**
```rust
pub fn auto_type_text(text: &str) -> Result<(), String>;
pub fn copy_to_clipboard(text: &str) -> Result<(), String>;
pub fn auto_output(text: &str) -> Result<OutputMethod, String>;

pub enum OutputMethod {
    Typed,
    Clipboard,
}
```

**Implementation:**
- Use `enigo` crate for keyboard simulation
- Character-by-character typing with 10ms delay
- Fallback to clipboard if typing fails
- Return which method was used (for notification)

**Edge Cases:**
- Special characters (handle properly with enigo)
- Very long text (>1000 chars) → clipboard only
- No focused window → clipboard

---

## 3. PYTHON SERVICE ARCHITECTURE

### 3.1 FastAPI Server (whisper_service.py)

```python
from fastapi import FastAPI, File, UploadFile, HTTPException
from faster_whisper import WhisperModel
import uvicorn
import os
import tempfile
import sys
import torch

app = FastAPI()
MODEL = None

@app.on_event("startup")
async def load_model():
    global MODEL
    device = "cuda" if torch.cuda.is_available() else "cpu"
    compute_type = "float16" if device == "cuda" else "int8"

    model_dir = os.path.join(os.getenv("APPDATA"), "Maata", "models")
    os.makedirs(model_dir, exist_ok=True)

    MODEL = WhisperModel(
        "medium",  # As per SPEC.md
        device=device,
        compute_type=compute_type,
        download_root=model_dir
    )

@app.post("/transcribe")
async def transcribe(audio: UploadFile = File(...)):
    if MODEL is None:
        raise HTTPException(status_code=500, detail="Model not loaded")

    # Save temp file
    with tempfile.NamedTemporaryFile(delete=False, suffix=".wav") as f:
        f.write(await audio.read())
        temp_path = f.name

    try:
        segments, info = MODEL.transcribe(
            temp_path,
            language=None,  # Auto-detect
            vad_filter=True,
            beam_size=5
        )

        text = " ".join([seg.text for seg in segments]).strip()

        return {
            "text": text,
            "language": info.language,
            "duration": info.duration
        }
    finally:
        os.remove(temp_path)

@app.get("/health")
async def health():
    return {"status": "ok", "model_loaded": MODEL is not None}

async def stdin_listener():
    """Listen for shutdown command"""
    loop = asyncio.get_event_loop()
    while True:
        line = await loop.run_in_executor(None, sys.stdin.readline)
        if line.strip() == "SHUTDOWN":
            os._exit(0)

if __name__ == "__main__":
    # Start stdin listener in background
    asyncio.create_task(stdin_listener())

    uvicorn.run(app, host="127.0.0.1", port=8765, log_level="error")
```

### 3.2 PyInstaller Configuration

Create `python/build.spec`:
```python
# -*- mode: python ; coding: utf-8 -*-

a = Analysis(
    ['whisper_service.py'],
    pathex=[],
    binaries=[],
    datas=[],
    hiddenimports=['uvicorn', 'fastapi', 'faster_whisper', 'torch'],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=None,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=None)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name='whisper_service',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    onefile=True,  # Single executable
)
```

Build command:
```bash
pyinstaller python/build.spec
```

---

## 4. IPC PROTOCOL

### 4.1 Endpoints

#### POST /transcribe
**Request:**
```
POST http://127.0.0.1:8765/transcribe
Content-Type: multipart/form-data

audio: <binary WAV file>
```

**Success Response (200):**
```json
{
  "text": "This is the transcribed text.",
  "language": "en",
  "duration": 5.2
}
```

**Error Response (500):**
```json
{
  "detail": "Error message here"
}
```

#### GET /health
**Request:**
```
GET http://127.0.0.1:8765/health
```

**Response (200):**
```json
{
  "status": "ok",
  "model_loaded": true
}
```

### 4.2 Error Codes
- **200:** Success
- **400:** Invalid audio file format
- **500:** Transcription failed or model not loaded
- **503:** Service unavailable

### 4.3 Timeout Strategy
- Health check: 500ms timeout, poll every 1s for 10s
- Transcription: 60s timeout (covers slow CPU scenarios)
- Connection timeout: 5s

### 4.4 Retry Logic
```rust
async fn transcribe_with_retry(audio_path: PathBuf, max_retries: u32) -> Result<String, String> {
    for attempt in 0..max_retries {
        match transcribe(audio_path.clone()).await {
            Ok(response) => return Ok(response.text),
            Err(e) if attempt < max_retries - 1 => {
                let delay = Duration::from_secs(2_u64.pow(attempt));
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

---

## 5. DATA FLOW DIAGRAMS

### 5.1 Success Flow

```
User Action                 Tauri                 Python              Output
───────────                ──────                ──────              ──────

1. Press Ctrl+Shift+Space
                    ──→  hotkey-pressed event
                    ──→  tray.update(Recording)
                    ──→  audio.start_recording()

2. Speak...
                    ──→  [CPAL captures samples]

3. Release key
                    ──→  hotkey-released event
                    ──→  audio.stop_recording()
                    ──→  [Saves temp.wav]
                    ──→  tray.update(Processing)
                    ──→  transcribe.transcribe(temp.wav)
                         │
                         └──→ HTTP POST /transcribe
                                        └──→ Model.transcribe()
                                        └──→ Return JSON ──→
                    ──→  [Receive response]
                    ──→  [Delete temp.wav]
                    ──→  typing.auto_output(text)
                    ──→  [Types text] ───────────────────────→ Text in app!
                    ──→  notification.show("Transcribed: ...")
                    ──→  tray.update(Idle)
```

### 5.2 Error Flow: Transcription Failed

```
User Action                 Tauri                 Python              Output
───────────                ──────                ──────              ──────

[Recording completed...]
                    ──→  transcribe.transcribe(temp.wav)
                         │
                         └──→ HTTP POST /transcribe
                                        └──→ Error: No speech
                                        └──→ 500 response ──→
                    ──→  [Receive error]
                    ──→  [Delete temp.wav]
                    ──→  notification.show("Transcription failed. Try again.")
                    ──→  tray.update(Idle)
```

### 5.3 Escape Key Cancellation

```
User Action                 Tauri                 Output
───────────                ──────                ──────

1. Press Ctrl+Shift+Space
                    ──→  [Recording started...]

2. Press Escape
                    ──→  escape-pressed event
                    ──→  audio.cancel_recording()
                    ──→  [Clear samples buffer]
                    ──→  [No file saved]
                    ──→  tray.update(Idle)
                    ──→  notification.show("Recording cancelled")
```

### 5.4 First-Run Model Download

```
User Action                 Tauri                 Python              UI
───────────                ──────                ──────              ──

1. Launch Maata.exe
                    ──→  main() starts
                    ──→  Check model exists?
                    ──→  [No model found]
                    ──→  Show "Setting up Maata" dialog ─────────→ [Dialog]
                    ──→  python_service.start()
                         └──→ Spawns Python
                                        └──→ WhisperModel("medium")
                                        └──→ [Downloads from HF]
                                        └──→ [Progress: 0-100%] ──→ [Progress bar]
                                        └──→ [Model loaded]
                                        └──→ /health returns 200
                    ──→  [Health check succeeds]
                    ──→  Close dialog ──────────────────────────→ [Tray only]
                    ──→  notification.show("Maata is ready!")
```

---

## 6. STATE MACHINE

### 6.1 Application States

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Initializing,   // Loading, starting Python service
    Idle,           // Ready to record
    Recording,      // Capturing audio
    Processing,     // Transcribing
    Error(String),  // Error state with message
}
```

### 6.2 State Transitions

```
                    ┌─────────────┐
                    │Initializing │
                    └──────┬──────┘
                           │ Python service ready
                           ▼
    ┌──────────────────────────────────────┐
    │              Idle                     │◄───────────┐
    │  Tray: White                         │            │
    └──────┬───────────────────────────────┘            │
           │ Hotkey pressed                             │
           ▼                                            │
    ┌──────────────────────────────────────┐            │
    │          Recording                    │            │
    │  Tray: Red                           │            │
    │  Overlay: Waveform                   │            │
    └──────┬───────────────────────────────┘            │
           │ Hotkey released OR 60s timeout             │
           ▼                                            │
    ┌──────────────────────────────────────┐            │
    │          Processing                   │            │
    │  Tray: Yellow                        │            │
    │  Overlay: "Transcribing..."          │            │
    └──────┬───────────────────────────────┘            │
           │ Success OR Error                           │
           └────────────────────────────────────────────┘

    Escape from Recording → Idle (immediate)
```

### 6.3 UI Changes Per State

| State | Tray Icon | Overlay | Notifications |
|-------|-----------|---------|---------------|
| Initializing | White | "Setting up Maata..." | None |
| Idle | White | Hidden | None |
| Recording | Red | "Recording..." + waveform | None |
| Processing | Yellow | "Transcribing..." | None |
| Idle (after success) | White | Hidden | "Transcribed: ..." |
| Error | White | Hidden | "Error: ..." |

---

## 7. CONFIGURATION MANAGEMENT

### 7.1 Settings Location
```
%APPDATA%\Maata\
├── settings.json       # User preferences
├── models/             # Whisper model files
│   └── medium/         # Auto-downloaded
└── history.db          # Transcription history (Phase 3)
```

### 7.2 Settings Schema

```json
{
  "version": "1.0",
  "hotkey": {
    "recording": "Ctrl+Shift+Space",
    "cancel": "Escape"
  },
  "model": {
    "size": "medium",
    "device": "auto"
  },
  "transcription": {
    "language": "auto",
    "vad_filter": true
  },
  "output": {
    "mode": "auto_type",
    "typing_speed_ms": 10
  },
  "ui": {
    "show_overlay": true,
    "notifications": true
  }
}
```

### 7.3 Default Values

```rust
impl Default for Settings {
    fn default() -> Self {
        Settings {
            hotkey_recording: "Ctrl+Shift+Space".to_string(),
            hotkey_cancel: "Escape".to_string(),
            model_size: ModelSize::Medium,
            language: "auto".to_string(),
            output_mode: OutputMode::AutoType,
            typing_speed_ms: 10,
            show_overlay: true,
            notifications: true,
        }
    }
}
```

---

## 8. SECURITY CONSIDERATIONS

### 8.1 Input Validation
- **Audio files:** Validate WAV format, max size 100MB
- **Hotkey strings:** Whitelist allowed keys/modifiers
- **Settings:** Validate JSON schema on load

### 8.2 Temp File Cleanup
```rust
struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
```
Use RAII to ensure temp files are deleted even on panic.

### 8.3 Audio Data Privacy
- Never send audio over network
- Delete WAV files immediately after transcription
- No logging of audio content
- No crash dumps containing audio data

### 8.4 No Secrets in Frontend
- All sensitive logic in Rust
- No API keys (using local models)
- Settings don't contain secrets

---

## 9. PERFORMANCE OPTIMIZATION

### 9.1 Model Caching
- Load model once on Python startup
- Keep in GPU/CPU memory throughout app lifetime
- Don't reload between transcriptions

### 9.2 Audio Buffer Sizing
- Buffer: 2048 samples (~128ms latency)
- Balances latency vs CPU usage
- Smaller buffer = lower latency but more CPU

### 9.3 HTTP Connection Pooling
```rust
lazy_static! {
    static ref CLIENT: reqwest::Client = reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(2)
        .build()
        .unwrap();
}
```

### 9.4 Async/Await Patterns
- All I/O operations async (HTTP, file writes)
- Use tokio runtime efficiently
- Don't block main thread

---

## 10. BUILD & PACKAGING

### 10.1 Development Build

```bash
# Terminal 1: Python service (for testing)
cd python
python -m venv venv
.\venv\Scripts\activate
pip install -r requirements.txt
python whisper_service.py

# Terminal 2: Tauri dev
npm run tauri dev
```

### 10.2 Production Build

```bash
# Step 1: Build Python executable
cd python
pyinstaller build.spec

# Step 2: Configure Tauri sidecar
# Edit tauri.conf.json:
{
  "bundle": {
    "externalBin": [
      "python/dist/whisper_service"
    ]
  }
}

# Step 3: Build Tauri
npm run tauri build
```

### 10.3 Tauri Configuration (tauri.conf.json)

```json
{
  "productName": "Maata",
  "version": "1.0.0",
  "identifier": "com.maata.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../dist"
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/icon.png"
    ],
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": ""
    },
    "externalBin": [
      "python/dist/whisper_service"
    ]
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "title": "Maata Settings",
        "width": 600,
        "height": 800,
        "visible": false,
        "skipTaskbar": true
      }
    ],
    "systemTray": {
      "iconPath": "icons/icon.png"
    }
  },
  "plugins": {
    "globalShortcut": {},
    "notification": {}
  }
}
```

### 10.4 Testing Strategy

**Unit Tests (Rust):**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_audio_recorder_start_stop() { ... }

    #[test]
    fn test_transcribe_api_format() { ... }

    #[test]
    fn test_settings_serialization() { ... }
}
```

**Integration Tests:**
- Mock Python service for HTTP testing
- Test full workflow with recorded audio samples
- Test error scenarios (service down, invalid audio, etc.)

**Manual Testing Checklist:**
- [ ] First launch downloads model
- [ ] Hotkey triggers recording
- [ ] Audio is captured (check WAV file temporarily)
- [ ] Transcription works on GPU/CPU
- [ ] Text is auto-typed correctly
- [ ] Escape cancels recording
- [ ] App survives Python service crash
- [ ] Settings persist across restarts

---

## END OF ARCHITECTURE

This architecture is designed to be:
- **Modular:** Clear separation of concerns
- **Robust:** Comprehensive error handling
- **Performant:** Meets all latency targets from SPEC.md
- **Maintainable:** Simple, well-documented patterns
- **Secure:** Privacy-first, no data leakage

Ready for implementation.
