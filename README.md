# Scribe

**Local, offline voice-to-text for Windows.**

![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)
![Platform: Windows](https://img.shields.io/badge/Platform-Windows-0078D6.svg)
![Tests: 151 passing](https://img.shields.io/badge/Tests-151%20passing-brightgreen.svg)

Scribe turns your voice into text without sending a single byte to the cloud. Press a hotkey, speak, and your words appear wherever your cursor is. It runs entirely on your machine using OpenAI's Whisper model, packaged as a single Windows executable with no accounts, no subscriptions, and no internet required after setup.

<!-- TODO: Add screenshot -->

## Features

| | Feature | Description |
|---|---|---|
| :microphone: | **Global Hotkey** | Press Ctrl+Shift+Space (customizable) to start/stop recording from anywhere |
| :bar_chart: | **Live Audio Level** | VU meter overlay confirms your mic is picking up sound |
| :scissors: | **Filler Word Removal** | Automatically strips "um", "uh", "you know", and other fillers |
| :speaker: | **Sound Effects** | Audio tones confirm when recording starts and stops |
| :rocket: | **Auto-Start** | Opt-in launch on Windows login |
| :scroll: | **Transcription History** | Browse and search your last 100 transcriptions |
| :keyboard: | **Customizable Hotkey** | Remap the recording toggle to any modifier+key combo |
| :wave: | **Welcome Onboarding** | First-run wizard walks you through model download and first dictation |
| :clipboard: | **Multiple Output Modes** | Clipboard+paste, clipboard only, or direct typing |
| :brain: | **Three Model Sizes** | Choose your trade-off between speed and accuracy |
| :no_entry_sign: | **Escape to Cancel** | Cancel a recording without transcribing |
| :computer: | **System Tray** | Runs quietly in the tray, always one click away |

## Quick Start

### Download a Release

1. Download the latest `.msi` installer from Releases
2. Install and launch Scribe
3. The welcome wizard will guide you through downloading a Whisper model
4. Press **Ctrl+Shift+Space** to start dictating

### Build from Source

See [Building from Source](#building-from-source) below.

## Usage

1. **Start Scribe** — it lives in the system tray
2. **Press Ctrl+Shift+Space** — the overlay appears with a live VU meter
3. **Speak** — Scribe records up to 65 seconds
4. **Press Ctrl+Shift+Space again** (or Escape to cancel) — recording stops, audio is transcribed locally
5. **Text appears** at your cursor position via paste or direct typing

That's the entire workflow. No browser tabs, no sign-in screens, no waiting for a server.

## Models

Models are downloaded on first use and stored in `%APPDATA%/Scribe/models/`.

| Model | Size | Speed | Languages | Best For |
|---|---|---|---|---|
| **base.en** | 148 MB | Fastest | English only | Quick notes, low-end hardware |
| **small.en** | 488 MB | Balanced | English only | Daily use, good accuracy |
| **large-v3-turbo-q5_0** | 1.0 GB | Slower | 100+ languages | Best accuracy, multilingual support |

All models are quantized GGML format sourced from Hugging Face.

## Settings

Settings are stored in `%APPDATA%/Scribe/settings.json` and are editable through the Settings UI (right-click the tray icon).

| Setting | Default | Options |
|---|---|---|
| **Hotkey** | Ctrl+Shift+Space | Any modifier+key combo |
| **Model** | base.en | base.en, small.en, large-v3-turbo-q5_0 |
| **Language** | Auto-detect | Auto or specific language code |
| **Output Mode** | Clipboard + Paste | Clipboard+Paste, Clipboard Only, Direct Typing |
| **Filler Removal** | On | On / Off |
| **Sound Effects** | On | On / Off |
| **Auto-Start** | Off | On / Off |

## Privacy

Scribe processes everything locally on your machine.

- **No network calls** during recording or transcription (the app works in airplane mode)
- **No telemetry**, no analytics, no crash reporting
- **No accounts** — there is nothing to sign up for
- **Audio is never saved to disk** — it stays in memory during transcription, then is discarded
- The only network request Scribe ever makes is to download a Whisper model from Hugging Face on first setup

Your voice data never leaves your device. Period.

## Building from Source

### Prerequisites

| Requirement | Install |
|---|---|
| **Rust** (edition 2021) | [rustup.rs](https://rustup.rs) |
| **LLVM / libclang** | `winget install LLVM.LLVM` |
| **CMake** | `winget install Kitware.CMake` |
| **MSVC Build Tools** | Visual Studio Build Tools with "Desktop development with C++" |
| **Node.js** | `winget install OpenJS.NodeJS.LTS` |

After installing LLVM, set the environment variable:

```
setx LIBCLANG_PATH "C:\Program Files\LLVM\bin"
```

### Build Commands

```bash
# Development (hot-reload)
cd src-tauri && cargo tauri dev

# Release build (produces installer in target/release/bundle/)
cd src-tauri && cargo tauri build

# Run tests (151 tests across 8 modules)
cd src-tauri && cargo test
```

## Architecture

```
Scribe.exe (single binary, ~50 MB idle RAM)
├── Main thread     — State machine, hotkey dispatch, tray, text insertion, post-processing
├── Audio thread    — WASAPI capture via cpal, 65s safety limit, mic volume auto-fix
├── Inference thread — whisper-rs (whisper.cpp) WhisperContext
└── WebView         — Overlay (VU meter), Settings UI (tabs, history, hotkey capture)
```

**Key design decisions:**

- **No Python, no sidecar processes** — Rust calls whisper.cpp directly through whisper-rs bindings
- **No HTTP IPC** — threads communicate via channels and shared atomic state
- **Lock-free audio levels** — `Arc<AtomicU32>` passes RMS from the audio callback to the UI at 10 Hz
- **Windows-native audio** — WASAPI via cpal with automatic mic mute detection and volume fix

**Tech stack:** Tauri 2.0, Rust, whisper-rs 0.15, cpal 0.15, enigo 0.2, arboard 3, tokio 1.0, regex 1, serde

**Tauri plugins:** single-instance, global-shortcut, notification, autostart

## Contributing

Contributions are welcome. Whether it's bug reports, feature requests, or pull requests — all help is appreciated.

1. Fork the repository
2. Create a feature branch
3. Make your changes and ensure `cargo test` passes
4. Submit a pull request

## License

MIT
