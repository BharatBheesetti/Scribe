# Phase 6: CI/CD Pipeline - Context

**Gathered:** 2026-02-16
**Status:** Ready for planning

<domain>
## Phase Boundary

Automated builds and releases on GitHub Actions. Push a semver tag, get a working NSIS installer as a GitHub Release with checksums. No multi-platform, no pre-releases, no manual workflow triggers for v1.1.

</domain>

<decisions>
## Implementation Decisions

### Release triggering
- Tag format: `v*.*.*` (semver) — e.g., v1.1.0, v1.2.3
- Stable releases only — no pre-release tags (v1.1.0-beta.1 does NOT trigger builds)
- Tags only — no workflow_dispatch manual trigger
- Auto-sync version from tag into Cargo.toml and tauri.conf.json before building — tag is single source of truth

### Build matrix & caching
- Windows x64 only — single target, no ARM64, no cross-platform
- Install LLVM + CMake fresh each run — no toolchain caching (reliability over speed)
- No cargo cache — clean builds every time (no stale cache risk)
- Runner selection: Claude's discretion (balance stability vs. maintenance)

### Release artifacts & notes
- Attach: NSIS installer (.exe) + SHA256 checksums file
- No portable zip, no debug symbols
- Installer filename: `Scribe-setup.exe` (no version in filename)
- Release notes: GitHub auto-generated from commit messages since last tag
- Auto-publish — tag push triggers build, release goes live immediately (no draft step)

### Workflow guardrails
- Tests must pass before building installer — cargo test gates the release
- Post-sync version verification — after writing tag version to config files, verify it matches before proceeding
- CI checks on push to main: Claude's discretion
- Failure notifications: GitHub default email notifications

### Claude's Discretion
- GitHub Actions runner version (windows-latest vs pinned)
- Whether to add a CI check on push to main (separate from release workflow)
- Exact workflow structure (single job vs multi-job)
- Caching strategy for LLVM/CMake download (if it helps without hurting reliability)
- Version sync implementation details (sed, PowerShell, dedicated action)

</decisions>

<specifics>
## Specific Ideas

- whisper.cpp build requires LLVM (libclang) + CMake + MSVC — these must be explicitly set up in the runner
- LIBCLANG_PATH must be set correctly for whisper-rs-sys compilation
- 151 existing tests must all pass as the gate
- Tauri v2 uses `tauri build` which produces the NSIS installer via bundler config

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 06-cicd-pipeline*
*Context gathered: 2026-02-16*
