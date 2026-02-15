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
| CICD-01 | Phase 6 | Pending |
| CICD-02 | Phase 6 | Pending |
| CICD-03 | Phase 6 | Pending |
| CICD-04 | Phase 6 | Pending |
| CICD-05 | Phase 6 | Pending |
| INST-01 | Phase 7 | Pending |
| INST-02 | Phase 7 | Pending |
| INST-03 | Phase 7 | Pending |
| INST-04 | Phase 7 | Pending |
| INST-05 | Phase 7 | Pending |
| WEB-01 | Phase 8 | Pending |
| WEB-02 | Phase 8 | Pending |
| WEB-03 | Phase 8 | Pending |
| WEB-04 | Phase 8 | Pending |
| WEB-05 | Phase 8 | Pending |

**Coverage:**
- v1.1 requirements: 15 total
- Mapped to phases: 15 (100% coverage)
- Phase 6 (CI/CD): 5 requirements
- Phase 7 (Installer): 5 requirements
- Phase 8 (Landing Page): 5 requirements

---
*Requirements defined: 2026-02-16*
*Last updated: 2026-02-16 after roadmap creation*
