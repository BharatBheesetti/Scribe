# Requirements: Scribe v1.1

**Defined:** 2026-02-16
**Core Value:** Voice-to-text that works instantly with zero setup, zero cloud dependency, and total privacy

## v1.1 Requirements

### CI/CD Pipeline

- [ ] **CICD-01**: GitHub Actions workflow builds Scribe on tag push (whisper.cpp + LLVM + CMake + MSVC)
- [ ] **CICD-02**: Workflow produces NSIS installer as build artifact
- [ ] **CICD-03**: Workflow publishes installer to GitHub Releases on tagged commits
- [ ] **CICD-04**: Rust/cargo build cache reduces CI build time
- [ ] **CICD-05**: Release includes changelog in release notes

### Installer

- [ ] **INST-01**: NSIS installer installs Scribe per-user (no admin required)
- [ ] **INST-02**: Installer downloads WebView2 runtime if not present (bootstrapper)
- [ ] **INST-03**: Installer creates Start Menu shortcut
- [ ] **INST-04**: Uninstaller cleanly removes Scribe (shortcuts, registry, app files)
- [ ] **INST-05**: Installer shows Scribe icon and branding

### Website

- [ ] **WEB-01**: GitHub Pages landing page with app description and download button
- [ ] **WEB-02**: Download button links to latest GitHub Release installer
- [ ] **WEB-03**: Page includes privacy statement (local-only, no cloud, no telemetry)
- [ ] **WEB-04**: Page includes features overview with screenshots
- [ ] **WEB-05**: Page is responsive (works on mobile)

## Future Requirements

### Distribution (deferred)

- **DIST-01**: Microsoft Store MSIX packaging and submission
- **DIST-02**: Code signing certificate (OV) for SmartScreen trust
- **DIST-03**: MSI installer for enterprise GPO deployment
- **DIST-04**: Auto-update via tauri-plugin-updater

## Out of Scope

| Feature | Reason |
|---------|--------|
| Code signing | No budget for certificate (~$279/yr) |
| Microsoft Store | Requires signing + dev account; deferred |
| MSI installer | NSIS sufficient for consumer distribution |
| Auto-updates | Deferred to future milestone |
| Offline WebView2 bundle | Bootstrapper sufficient; saves 127MB |
| Telemetry / analytics | Core value is privacy-first |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CICD-01 | — | Pending |
| CICD-02 | — | Pending |
| CICD-03 | — | Pending |
| CICD-04 | — | Pending |
| CICD-05 | — | Pending |
| INST-01 | — | Pending |
| INST-02 | — | Pending |
| INST-03 | — | Pending |
| INST-04 | — | Pending |
| INST-05 | — | Pending |
| WEB-01 | — | Pending |
| WEB-02 | — | Pending |
| WEB-03 | — | Pending |
| WEB-04 | — | Pending |
| WEB-05 | — | Pending |

**Coverage:**
- v1.1 requirements: 15 total
- Mapped to phases: 0
- Unmapped: 15 (pending roadmap)

---
*Requirements defined: 2026-02-16*
*Last updated: 2026-02-16 after user scoping*
