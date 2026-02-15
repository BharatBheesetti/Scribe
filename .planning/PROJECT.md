# Scribe

## What This Is

Scribe is a local-first voice-to-text desktop app for Windows. Press a hotkey, speak, and your words appear as text wherever your cursor is — no cloud, no subscriptions, no data leaving your machine. Built with Tauri v2 (Rust) and whisper.cpp for on-device inference.

## Core Value

Voice-to-text that works instantly with zero setup, zero cloud dependency, and total privacy — press hotkey, speak, text appears.

## Requirements

### Validated

- Recording: User can press hotkey to start/stop voice recording (v1.0)
- Transcription: Speech is transcribed locally via whisper.cpp models (v1.0)
- Text insertion: Transcribed text is pasted at cursor position (v1.0)
- Model management: User can download whisper models from within the app (v1.0)
- Overlay: Visual feedback pill follows cursor during recording (v1.0)
- Audio level: VU meter shows mic input level in overlay (v1.0)
- Filler removal: "um", "uh", "you know" etc. stripped from output (v1.0)
- Sound effects: Audio cues for recording start/stop (v1.0)
- Custom hotkey: User can change the recording hotkey (v1.0)
- Auto-start: App can launch on Windows login (v1.0)
- History: Past transcriptions saved and searchable (v1.0)
- Onboarding: First-run wizard guides model download and first use (v1.0)
- Settings: Language, output mode, model size, toggles for features (v1.0)
- System tray: App lives in tray, minimal footprint (v1.0)
- Single instance: Only one copy of Scribe runs at a time (v1.0)

### Active — Milestone v1.1: Packaging & Distribution

- [ ] GitHub Actions CI/CD pipeline (build, cache, release on tag push)
- [ ] NSIS installer (per-user, WebView2 bootstrapper, branding)
- [ ] GitHub Pages landing page (download links, privacy, features)

### Out of Scope

- Cloud transcription — Core value is local/private
- macOS/Linux — Windows-first, cross-platform later
- Real-time streaming transcription — Batch after recording for quality
- Plugin/extension system — Keep it simple
- Code signing — No budget for certificate (~$279/yr)
- Microsoft Store — Requires signing + dev account; deferred
- MSI installer — NSIS sufficient for consumer distribution
- Auto-updates — Ship installer first, add later

## Context

- Built with Tauri v2, Rust backend, Vanilla JS frontend
- whisper-rs (whisper.cpp bindings) for inference — no Python dependency
- WASAPI audio capture via cpal, with COM-based mic volume auto-fix
- 151 tests passing across 8 modules
- All Phase 2A features shipped and working
- App runs as single .exe with WebView2 runtime

## Constraints

- **Platform**: Windows-only (WASAPI, Win32 APIs for caret detection, PlaySoundA)
- **Privacy**: All processing must remain on-device, no network calls except model download
- **Architecture**: Single binary, no external service dependencies
- **Stack**: Tauri v2 + Rust + whisper-rs — no Python, no Node runtime at execution

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| whisper-rs over Python faster-whisper | Single binary, no Python dep, simpler packaging | Good |
| Tauri v2 over Electron | Smaller binary, Rust backend, native performance | Good |
| Toggle hotkey model (not push-to-talk) | RegisterHotKey can't detect key release on Windows | Good |
| Programmatic WAV gen over audio files | No asset files to bundle, smaller binary | Good |
| Frontend-only history search | Max 100 entries, client-side filtering fast enough | Good |
| Dynamic hotkey registration | Register new before unregister old — prevents race condition | Good |
| OV over EV cert (deferred) | March 2024 SmartScreen policy change — EV no longer bypasses warnings | — Pending |
| NSIS over MSI | Consumer-friendly, auto-updater ready, per-user install | Good |
| WebView2 bootstrapper over offline bundle | Most Win10/11 have it; saves 127MB installer size | Good |
| Skip code signing for v1.1 | No budget; SmartScreen "Run anyway" acceptable for indie app | — Pending |

---
*Last updated: 2026-02-16 after v1.1 requirements scoping*
