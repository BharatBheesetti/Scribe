# Roadmap: Scribe

## Milestones

- ✅ **v1.0 Core App + Essential Features** - Phases 1-5 (shipped 2026-02-13)
- 🚧 **v1.1 Packaging & Distribution** - Phases 6-8 (in progress)

## Phases

<details>
<summary>✅ v1.0 Core App + Essential Features (Phases 1-5) - SHIPPED 2026-02-13</summary>

**Milestone Goal:** Voice-to-text that works instantly with zero setup, zero cloud dependency, and total privacy — press hotkey, speak, text appears.

### Phase 0: Foundation
**Goal**: Core architecture and subsystems operational
**Status**: Complete

### Phase 1: Integration & Polish
**Goal**: End-to-end recording → transcription → paste flow
**Status**: Complete

### Phase 2A: 8 Essential Features
**Goal**: Audio level, filler removal, sound effects, custom hotkey, auto-start, history search, onboarding, README
**Status**: Complete (Waves 1-4 shipped)

**Tests:** 151 passing
**Build:** Zero errors, zero warnings

</details>

### 🚧 v1.1 Packaging & Distribution (In Progress)

**Milestone Goal:** Professional distribution channels with automated builds, installer, and landing page for public release.

#### Phase 6: CI/CD Pipeline
**Goal**: Automated builds and releases on GitHub Actions with proper caching and artifacts
**Depends on**: Phase 5 (v1.0 complete)
**Requirements**: CICD-01, CICD-02, CICD-03, CICD-04, CICD-05
**Success Criteria** (what must be TRUE):
  1. Push a version tag (v*.*.*) triggers GitHub Actions workflow that builds Scribe successfully
  2. Workflow compiles whisper.cpp with LLVM/CMake/MSVC toolchain on clean runner environment
  3. Workflow produces NSIS installer as downloadable artifact
  4. Tagged commits automatically create GitHub Release with installer attached
  5. Rust/cargo build cache reduces whisper.cpp compile time from 15+ minutes to under 5 minutes
  6. Release notes include changelog extracted from commits or CHANGELOG.md
**Plans**: 2 plans

Plans:
- [ ] 06-01-PLAN.md — Create release workflow and configure Tauri for CI builds
- [ ] 06-02-PLAN.md — End-to-end pipeline verification (tag push, workflow run, release check)

#### Phase 7: Installer
**Goal**: Professional NSIS installer with branding, WebView2 bootstrapper, and clean install/uninstall
**Depends on**: Phase 6 (CI/CD must produce installer artifacts)
**Requirements**: INST-01, INST-02, INST-03, INST-04, INST-05
**Success Criteria** (what must be TRUE):
  1. User can install Scribe without admin rights (per-user install to %LOCALAPPDATA%)
  2. Installer automatically downloads and installs WebView2 runtime if not already present
  3. Start Menu shortcut exists after installation and launches Scribe correctly
  4. Uninstaller removes all app files, shortcuts, and registry entries (verified on clean Windows VM)
  5. Installer displays Scribe icon and branding throughout install wizard
**Plans**: TBD

Plans:
- [ ] 07-01: [Description TBD during plan-phase]

#### Phase 8: Landing Page
**Goal**: GitHub Pages website with download links, privacy statement, features overview, and mobile responsiveness
**Depends on**: Phase 6 (download links require GitHub Releases)
**Requirements**: WEB-01, WEB-02, WEB-03, WEB-04, WEB-05
**Success Criteria** (what must be TRUE):
  1. Landing page lives at https://{username}.github.io/scribe_final/ with hero section and download CTA
  2. Download button dynamically links to latest GitHub Release installer (no hardcoded URLs)
  3. Privacy statement clearly explains local-only processing, no cloud, no telemetry, mic/clipboard access
  4. Features overview includes screenshots showing overlay, VU meter, history, settings, onboarding
  5. Page renders correctly on mobile devices (responsive design, tested at 375px and 768px widths)
**Plans**: TBD

Plans:
- [ ] 08-01: [Description TBD during plan-phase]

## Progress

**Execution Order:**
Phases execute in numeric order: 6 → 7 → 8

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1-5. v1.0 | v1.0 | All | Complete | 2026-02-13 |
| 6. CI/CD Pipeline | v1.1 | 0/2 | Planning complete | - |
| 7. Installer | v1.1 | 0/? | Not started | - |
| 8. Landing Page | v1.1 | 0/? | Not started | - |

---
*Last updated: 2026-02-16 after Phase 6 planning*
