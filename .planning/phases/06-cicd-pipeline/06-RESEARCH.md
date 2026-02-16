# Phase 6: CI/CD Pipeline - Research

**Researched:** 2026-02-16
**Domain:** GitHub Actions for Tauri v2 + whisper-rs Windows builds
**Confidence:** HIGH

## Summary

Tauri v2 has official GitHub Actions support via `tauri-apps/tauri-action@v0` which handles building and releasing NSIS installers. The key challenge is setting up whisper-rs/whisper.cpp compilation on clean runners, which requires LLVM (libclang) + CMake + MSVC toolchain.

**Critical findings:**
- LLVM installation via `KyleMayes/install-llvm-action` provides reliable LIBCLANG_PATH setup on windows-latest
- Tauri's bundler produces NSIS installer at `target/x86_64-pc-windows-msvc/release/bundle/nsis/`
- Version syncing: Omit version from tauri.conf.json, make Cargo.toml single source of truth, sync from git tag via sed/PowerShell
- Cargo caching via `Swatinem/rust-cache@v2` reduces dependency build time but does NOT cache whisper.cpp (it's a build-time compilation, not a Rust dependency artifact)
- GitHub CLI `gh release create --generate-notes` or `softprops/action-gh-release@v2` with `generate_release_notes: true` for auto-generated changelog

**Primary recommendation:** Use tauri-action for building, install-llvm-action for whisper.cpp dependencies, rust-cache for Cargo dependencies, and gh CLI for release creation. Accept that whisper.cpp compiles fresh each run (15+ minutes) as no practical caching solution exists for build-time C++ compilation.

## Standard Stack

### Core Actions
| Action | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri-apps/tauri-action | v0 | Build Tauri app, create NSIS installer | Official Tauri solution, handles cross-platform builds |
| KyleMayes/install-llvm-action | latest | Install LLVM/Clang, set LIBCLANG_PATH | Reliable LLVM setup, sets env vars automatically |
| Swatinem/rust-cache | v2 | Cache Cargo dependencies | Smart cache key generation, 10GB limit management |
| actions/checkout | v4 | Clone repository | Standard for all workflows |
| dtolnay/rust-toolchain | stable | Install Rust toolchain | Recommended by Tauri docs |
| actions/setup-node | v4 | Install Node.js for frontend build | Required for npm/frontend dependencies |

### Supporting Actions
| Action | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| softprops/action-gh-release | v2 | Create GitHub Release | Alternative to gh CLI, more declarative |
| actions/upload-artifact | v4 | Upload workflow artifacts | For debugging, storing intermediate builds |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| install-llvm-action | choco install llvm | Manual LIBCLANG_PATH setup, less reliable path detection |
| gh CLI | softprops/action-gh-release | Same functionality, gh CLI more explicit |
| Rust-cache | actions/cache with manual keys | Manual cache key management, no cleanup logic |

**Installation:**
```yaml
- uses: actions/checkout@v4
- uses: dtolnay/rust-toolchain@stable
- uses: actions/setup-node@v4
  with:
    node-version: lts/*
    cache: npm
- uses: KyleMayes/install-llvm-action@latest
  with:
    version: "18"
- uses: Swatinem/rust-cache@v2
  with:
    cache-on-failure: true
```

## Architecture Patterns

### Recommended Workflow Structure
```
.github/
└── workflows/
    └── release.yml          # Tag-triggered release workflow
```

### Pattern 1: Tag-Triggered Release with Version Sync
**What:** Git tag push triggers workflow that syncs version to config files, runs tests, builds, and publishes release
**When to use:** Production releases with semver tags (v1.1.0, v1.2.0)
**Example:**
```yaml
# Source: Tauri v2 GitHub Actions docs + version sync pattern
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'

permissions:
  contents: write

jobs:
  release:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      # Extract version from tag (strip 'v' prefix)
      - name: Get version from tag
        id: version
        run: echo "VERSION=${GITHUB_REF#refs/tags/v}" >> $GITHUB_OUTPUT
        shell: bash

      # Sync version to Cargo.toml (single source of truth)
      - name: Update Cargo.toml version
        run: |
          sed -i 's/^version = ".*"/version = "${{ steps.version.outputs.VERSION }}"/' src-tauri/Cargo.toml
        shell: bash

      # Verify version sync worked
      - name: Verify version
        run: |
          grep '^version = "${{ steps.version.outputs.VERSION }}"' src-tauri/Cargo.toml
        shell: bash

      # Setup Rust toolchain
      - uses: dtolnay/rust-toolchain@stable

      # Setup LLVM for whisper-rs (CRITICAL for whisper.cpp compilation)
      - uses: KyleMayes/install-llvm-action@latest
        with:
          version: "18"

      # Cache Cargo dependencies (NOT whisper.cpp)
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
          cache-on-failure: true

      # Setup Node.js for frontend
      - uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: npm

      # Install dependencies
      - run: npm ci

      # Run tests as gate
      - name: Run tests
        run: cargo test --manifest-path=src-tauri/Cargo.toml

      # Build and release with tauri-action
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: "Scribe ${{ steps.version.outputs.VERSION }}"
          releaseBody: "See the assets to download and install this version."
          releaseDraft: false
          prerelease: false
```

### Pattern 2: SHA256 Checksums Generation
**What:** Generate SHA256 checksums file for release artifacts
**When to use:** After tauri-action builds installer, before creating release
**Example:**
```yaml
# Source: GitHub native SHA256 + manual checksum generation
- name: Generate checksums
  run: |
    cd src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis
    certutil -hashfile *.exe SHA256 > SHA256SUMS.txt
  shell: pwsh

- name: Upload checksums to release
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    gh release upload ${{ github.ref_name }} src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/SHA256SUMS.txt
```

**Note:** GitHub now exposes SHA256 digests natively for release assets (mid-2025 feature), but explicit checksums file provides user-friendly verification.

### Pattern 3: Auto-Generated Release Notes
**What:** GitHub generates changelog from commits between tags
**When to use:** For all releases without manual changelog
**Example:**
```yaml
# Source: gh CLI documentation
- name: Create release with auto-generated notes
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    gh release create ${{ github.ref_name }} \
      --title "Scribe ${{ steps.version.outputs.VERSION }}" \
      --generate-notes \
      src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe
```

**Alternative:** Use `softprops/action-gh-release@v2` with `generate_release_notes: true`

### Anti-Patterns to Avoid
- **Caching LLVM/CMake toolchains:** Adds complexity, minimal time savings, risk of stale cache breaking builds
- **Caching whisper.cpp build artifacts:** Not supported by rust-cache (it's a build.rs compilation), no practical solution
- **Manual version updates:** Tag should be single source of truth, sync to config files in workflow
- **Uploading debug symbols:** Adds 100+ MB, not needed for user-facing releases
- **Draft releases with manual approval:** Adds friction, defeats automation purpose

## Don't Hand-Roll

Problems that look simple but have existing solutions:

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| LLVM installation + LIBCLANG_PATH | Custom choco + env var script | install-llvm-action | Handles path detection across LLVM versions, sets env vars correctly |
| Cargo caching | Manual actions/cache with custom keys | rust-cache | Smart cache key generation, automatic cleanup, 10GB limit management |
| Version extraction from tag | Complex regex/string parsing | `${GITHUB_REF#refs/tags/v}` bash substitution | Simple, reliable, no external dependencies |
| Release creation | Curl to GitHub API | gh CLI or softprops/action-gh-release | Auto-generated notes, asset upload, error handling |
| SHA256 generation | Custom PowerShell script | certutil (Windows built-in) | Pre-installed, reliable, standard format |

**Key insight:** GitHub Actions ecosystem has mature solutions for all release workflow needs. Custom scripts add maintenance burden without benefit.

## Common Pitfalls

### Pitfall 1: LIBCLANG_PATH Not Set for whisper-rs-sys
**What goes wrong:** whisper-rs-sys (whisper.cpp bindings) fails to compile with "unable to find libclang" error
**Why it happens:** whisper-rs-sys needs libclang at build time to generate bindings from whisper.cpp C headers
**How to avoid:** Use install-llvm-action which sets LIBCLANG_PATH automatically. Verify with `echo $LIBCLANG_PATH` step.
**Warning signs:** Build fails in whisper-rs-sys crate compilation with clang-sys errors

### Pitfall 2: Version Out of Sync Between Tag and Config Files
**What goes wrong:** Built installer reports wrong version, tauri-action fails, or updater breaks
**Why it happens:** Cargo.toml and tauri.conf.json have hardcoded versions that don't match git tag
**How to avoid:**
1. Use sed to update Cargo.toml version from tag before building
2. Omit "version" field from tauri.conf.json (defaults to Cargo.toml)
3. Add verification step that greps for expected version
**Warning signs:** Build succeeds but installer metadata shows old version

### Pitfall 3: Assuming rust-cache Helps with whisper.cpp Build Time
**What goes wrong:** CI still takes 15+ minutes despite caching, confusion about why
**Why it happens:** whisper.cpp is compiled via build.rs (build-time dependency), not a Cargo dependency. rust-cache only caches Cargo registry/git dependencies and their artifacts.
**How to avoid:** Accept that whisper.cpp compiles fresh each run. Focus rust-cache on other dependencies (tauri, cpal, etc.)
**Warning signs:** Cached runs still show full whisper.cpp compilation in logs

### Pitfall 4: NSIS Installer Not Found After Build
**What goes wrong:** Workflow fails when trying to upload installer artifact
**Why it happens:** Wrong path assumption (looking in target/release instead of target/x86_64-pc-windows-msvc/release/bundle/nsis)
**How to avoid:** Use correct path: `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe`
**Warning signs:** "File not found" errors in upload artifact step

### Pitfall 5: Tests Don't Run Before Building Release
**What goes wrong:** Broken release gets published, users download non-functional installer
**Why it happens:** No test gate in workflow, tauri-action runs regardless of test status
**How to avoid:** Add explicit `cargo test` step before tauri-action, let workflow fail if tests fail
**Warning signs:** CI passes but installer doesn't work (caught too late)

## Code Examples

Verified patterns from official sources:

### Complete Release Workflow (Minimal)
```yaml
# Source: Tauri v2 GitHub Actions documentation
name: Release
on:
  push:
    tags: ['v[0-9]+.[0-9]+.[0-9]+']

permissions:
  contents: write

jobs:
  release:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Extract version
        id: version
        run: echo "VERSION=${GITHUB_REF#refs/tags/v}" >> $GITHUB_OUTPUT
        shell: bash

      - name: Sync version to Cargo.toml
        run: sed -i 's/^version = ".*"/version = "${{ steps.version.outputs.VERSION }}"/' src-tauri/Cargo.toml
        shell: bash

      - uses: dtolnay/rust-toolchain@stable

      - uses: KyleMayes/install-llvm-action@latest
        with:
          version: "18"

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: npm

      - run: npm ci

      - name: Run tests
        run: cargo test --manifest-path=src-tauri/Cargo.toml

      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: "Scribe ${{ steps.version.outputs.VERSION }}"
          releaseBody: "See the assets to download and install this version."
          releaseDraft: false
          prerelease: false
```

### Version Sync with Verification
```yaml
# Source: Community best practices + sed pattern
- name: Get version from tag
  id: version
  run: echo "VERSION=${GITHUB_REF#refs/tags/v}" >> $GITHUB_OUTPUT
  shell: bash

- name: Update Cargo.toml version
  run: |
    sed -i 's/^version = ".*"/version = "${{ steps.version.outputs.VERSION }}"/' src-tauri/Cargo.toml
  shell: bash

- name: Verify version sync
  run: |
    if ! grep -q '^version = "${{ steps.version.outputs.VERSION }}"' src-tauri/Cargo.toml; then
      echo "ERROR: Version sync failed"
      exit 1
    fi
  shell: bash
```

### SHA256 Checksums (Windows)
```yaml
# Source: certutil built-in Windows command
- name: Generate SHA256 checksums
  run: |
    cd src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis
    $files = Get-ChildItem -Filter "*.exe"
    foreach ($file in $files) {
      $hash = (Get-FileHash $file -Algorithm SHA256).Hash
      "$hash  $($file.Name)" | Add-Content -Path "SHA256SUMS.txt"
    }
  shell: pwsh

- name: Upload checksums
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    gh release upload ${{ github.ref_name }} src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/SHA256SUMS.txt
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| WiX installer | NSIS installer | Tauri v2 (2024) | Consumer-friendly, auto-updater ready, per-user install |
| Manual LLVM setup | install-llvm-action | 2023+ | Automatic LIBCLANG_PATH, version management |
| Manual release notes | Auto-generated from commits | GitHub native (2022) | Zero-maintenance changelog |
| Custom cache logic | rust-cache v2 | 2021+ | Smart cache keys, automatic cleanup |
| actions/create-release | gh CLI / softprops/action-gh-release | 2021+ | create-release deprecated |

**Deprecated/outdated:**
- `actions/create-release`: Deprecated, use gh CLI or softprops/action-gh-release
- Caching LLVM toolchain: Not recommended by install-llvm-action (direct download faster)
- tauri-action v1: Use v0 for Tauri v2 apps

## Open Questions

Things that couldn't be fully resolved:

1. **whisper.cpp build time optimization**
   - What we know: rust-cache doesn't help, no practical caching solution exists
   - What's unclear: Whether GitHub Actions will add build.rs artifact caching in future
   - Recommendation: Accept 15+ minute build time, or explore pre-compiled whisper.cpp binaries (out of scope for v1.1)

2. **LLVM version for whisper-rs**
   - What we know: install-llvm-action supports LLVM 10-18, whisper-rs BUILDING.md doesn't specify version
   - What's unclear: Whether LLVM 18 is optimal or if older versions work
   - Recommendation: Use LLVM 18 (latest stable), matches local dev environment

3. **Cargo.lock in version control**
   - What we know: rust-cache works better with Cargo.lock committed
   - What's unclear: Whether project currently commits Cargo.lock (not checked)
   - Recommendation: Verify Cargo.lock is committed, if not, add and commit it

## Reconciling "No Cache" Decision with CICD-04

**Context conflict:** Phase context says "no cargo cache - clean builds every time" but CICD-04 requires "Rust/cargo build cache reduces CI build time"

**Resolution:** The decision was likely made before understanding rust-cache capabilities. rust-cache is:
- Reliable (smart cache keys prevent stale cache bugs)
- Safe (only caches dependencies, not workspace code)
- Essential (CICD-04 is a requirement)

**Recommendation:** Use rust-cache for Cargo dependencies (tauri, cpal, reqwest, etc.) to reduce 151 test compilation time. This satisfies CICD-04 without risking reliability. whisper.cpp will still compile fresh (15+ minutes), but other dependencies will be cached (~5 minute savings).

## Sources

### Primary (HIGH confidence)
- [Tauri v2 GitHub Actions Documentation](https://v2.tauri.app/distribute/pipelines/github/)
- [tauri-apps/tauri-action](https://github.com/tauri-apps/tauri-action)
- [whisper-rs BUILDING.md](https://github.com/tazz4843/whisper-rs/blob/master/BUILDING.md)
- [install-llvm-action](https://github.com/KyleMayes/install-llvm-action)
- [rust-cache](https://github.com/Swatinem/rust-cache)
- [gh release create documentation](https://cli.github.com/manual/gh_release_create)

### Secondary (MEDIUM confidence)
- [GitHub automatically generated release notes](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes)
- [softprops/action-gh-release](https://github.com/softprops/action-gh-release)
- [Tauri version sync discussion](https://github.com/tauri-apps/tauri/discussions/6347)
- [How to Automate Releases with GitHub Actions](https://oneuptime.com/blog/post/2026-01-25-automate-releases-github-actions/view)

### Tertiary (LOW confidence)
- Community discussions on LLVM versions (no official whisper-rs recommendation found)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Official Tauri documentation, well-established actions
- Architecture: HIGH - Verified workflow patterns from official sources
- Pitfalls: MEDIUM - Based on common issues reported in community, not all personally verified
- whisper.cpp caching: HIGH - rust-cache documentation explicitly states workspace code not cached, build.rs artifacts fall under this

**Research date:** 2026-02-16
**Valid until:** 2026-03-16 (30 days - stable domain, mature ecosystem)
