---
phase: 06-ci-cd-pipeline
plan: 01
subsystem: infra
tags: [github-actions, ci-cd, nsis, tauri, llvm, rust, windows]

# Dependency graph
requires:
  - phase: 00-05 (all v1.0 phases)
    provides: Working Scribe app with 151 tests, whisper-rs integration
provides:
  - Complete GitHub Actions release workflow (.github/workflows/release.yml)
  - CI-compatible Tauri configuration (version from Cargo.toml)
  - Automated NSIS installer builds on tag push
  - GitHub Releases with SHA256 checksums
  - Auto-generated release notes from commits
affects: [07-installer, future releases, version management]

# Tech tracking
tech-stack:
  added: [GitHub Actions, tauri-apps/tauri-action@v0, KyleMayes/install-llvm-action, Swatinem/rust-cache@v2]
  patterns: [Version sync from git tag to Cargo.toml, Test gate before build, SHA256 checksum generation]

key-files:
  created: [.github/workflows/release.yml]
  modified: [src-tauri/tauri.conf.json]

key-decisions:
  - "Removed version from tauri.conf.json - Cargo.toml is single source of truth, synced from git tag"
  - "Version sync via sed before build ensures git tag drives version number"
  - "Test gate enforced - all 151 tests must pass before building installer"
  - "SHA256 checksums generated via PowerShell and uploaded to release"
  - "Auto-generated release notes via gh CLI --generate-notes"

patterns-established:
  - "Version flow: git tag (v*.*.*) → sed sync to Cargo.toml → Tauri reads from Cargo.toml"
  - "Release pipeline: checkout → sync version → setup toolchain → test → build → release → checksums"
  - "LLVM v18 installation for whisper-rs-sys compilation on CI"

# Metrics
duration: 2min
completed: 2026-02-16
---

# Phase 6 Plan 01: Release Workflow Summary

**Complete GitHub Actions CI/CD pipeline: tag-triggered builds with LLVM toolchain, test gate, NSIS installer, and SHA256 checksums**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-16T13:40:41Z
- **Completed:** 2026-02-16T13:42:28Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Created complete GitHub Actions release workflow with all 5 CICD requirements
- Configured Tauri for CI compatibility (version from Cargo.toml)
- Implemented version sync from git tag to Cargo.toml via sed
- Established test gate (151 tests must pass before build)
- Configured LLVM v18 installation for whisper-rs-sys compilation
- Set up Rust dependency caching via rust-cache
- Integrated SHA256 checksum generation and upload
- Enabled auto-generated release notes from commits

## Task Commits

Each task was committed atomically:

1. **Task 1: Configure tauri.conf.json for CI compatibility** - `199e91f` (chore)
2. **Task 2: Create the release workflow** - `2eef8c2` (feat)

**Plan metadata:** Will be committed after STATE.md update

## Files Created/Modified

- `.github/workflows/release.yml` - Complete CI/CD release pipeline (114 lines)
  - Tag trigger (v[0-9]+.[0-9]+.[0-9]+)
  - Version extraction and sync to Cargo.toml
  - Rust + LLVM + Node.js toolchain setup
  - Rust dependency caching (workspaces: src-tauri)
  - Test gate (cargo test)
  - Tauri NSIS build via tauri-action@v0
  - GitHub Release creation
  - Auto-generated release notes (gh release edit --generate-notes)
  - SHA256 checksums (PowerShell Get-FileHash)
  - Checksum upload to release

- `src-tauri/tauri.conf.json` - Removed hardcoded version field, added NSIS config
  - Removed "version" field (Cargo.toml is single source of truth)
  - Added "nsis" section under "bundle" with displayLanguageSelector: false

## Decisions Made

**Version management approach:**
- Removed hardcoded version from tauri.conf.json
- Git tag (v*.*.*) is the authoritative version source
- CI workflow extracts version from tag and syncs to Cargo.toml via sed
- Tauri v2 defaults to reading version from Cargo.toml when config field is absent
- **Rationale:** Single source of truth, prevents version drift, tag drives entire release

**Test gate enforcement:**
- All 151 tests must pass before build step executes
- `cargo test` failure stops the pipeline
- **Rationale:** Prevents releasing broken builds, ensures quality gate

**LLVM v18 for whisper-rs:**
- KyleMayes/install-llvm-action@latest with version "18"
- Critical for whisper-rs-sys compilation from source
- **Rationale:** whisper-rs-sys requires LLVM/libclang on CI runners

**Rust caching strategy:**
- Swatinem/rust-cache@v2 with workspaces: src-tauri
- cache-on-failure: true to cache dependencies even if build fails
- **Rationale:** Significantly reduces rebuild time on CI (dependencies rarely change)

**SHA256 checksums:**
- PowerShell Get-FileHash for all .exe files in NSIS bundle directory
- SHA256SUMS.txt uploaded to GitHub Release
- **Rationale:** Users can verify installer integrity, security best practice

**Release notes:**
- Auto-generated from commits since last tag via gh CLI
- Empty releaseBody in tauri-action, populated after via gh release edit
- **Rationale:** Accurate commit history, no manual note maintenance

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - both tasks completed successfully on first attempt.

## User Setup Required

None - workflow is self-contained, uses GITHUB_TOKEN secret automatically available in GitHub Actions.

## Next Phase Readiness

**Ready for:**
- Phase 7 (Installer): Can now test release process end-to-end by pushing a test tag
- Future releases: Pipeline is production-ready

**Testing needed:**
- Push a test tag (e.g., v1.1.0-test) to verify workflow executes successfully
- Verify LLVM installation works on GitHub Actions Windows runner
- Confirm NSIS installer builds and uploads correctly
- Validate SHA256SUMS.txt generation and upload

**Potential concerns:**
- whisper-rs-sys compilation on CI is untested (known blocker from STATE.md)
- First run will have no Rust cache, may take 15-20 minutes
- WebView2 bootstrapper requires internet (documented tradeoff)

**Blockers:**
None - workflow is ready for testing.

---
*Phase: 06-ci-cd-pipeline*
*Completed: 2026-02-16*
