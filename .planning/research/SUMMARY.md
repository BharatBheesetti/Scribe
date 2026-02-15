# Project Research Summary

**Project:** Scribe v1.1 Packaging & Distribution
**Domain:** Windows Desktop Application Distribution
**Researched:** 2026-02-16
**Confidence:** HIGH

## Executive Summary

This research synthesizes packaging and distribution strategies for Scribe, a local-first voice-to-text desktop application built with Tauri v2. The milestone focuses on five interconnected distribution channels: signed installers (MSI + NSIS), Microsoft Store listing, CI/CD automation, code signing infrastructure, and a landing page. The recommended approach prioritizes direct downloads first (weeks 1-3) and defers Microsoft Store to a second wave (weeks 4-5) due to manual MSIX packaging requirements and Store review timelines.

The critical architectural decision is to use OV code signing certificates ($200-380/year) instead of EV ($300-500/year), since Microsoft's March 2024 SmartScreen policy change eliminated EV's instant reputation advantage. Both certificate types now build reputation organically through download telemetry, making OV the cost-effective choice. Expect 2-4 weeks of SmartScreen warnings for new users despite signing; mitigate with soft launch to trusted users first. The CI/CD pipeline must handle whisper.cpp's full C++ toolchain requirements (MSVC, CMake, LLVM) on GitHub Actions, with build caching reducing compile time from 15 minutes to 3 minutes.

Key risks: (1) Tauri v2 doesn't support native MSIX generation, requiring manual Windows SDK tooling for Microsoft Store; (2) SmartScreen reputation building takes weeks regardless of certificate type; (3) timestamp server failures can break 5-15% of builds without retry logic; (4) dual distribution channels (Store vs direct) can drift without synchronized release processes. Mitigation strategies are well-documented in PITFALLS.md and integrated into phase recommendations below.

## Key Findings

### Recommended Stack

Tauri v2's native bundlers (WiX for MSI, NSIS for EXE) handle most packaging needs without third-party tools. Code signing integrates cleanly via `tauri.conf.json` with support for both traditional PFX certificates and Azure Key Vault. GitHub Actions provides all required toolchain components (MSVC, CMake, Windows SDK) on `windows-latest` runners, with LLVM available as a Visual Studio component. Microsoft Store submission requires manual MSIX packaging using Windows SDK's `makeappx.exe` until Tauri adds native support (tracked in GitHub issue #8548).

**Core technologies:**
- **WiX + NSIS (Tauri bundlers)**: Dual installer strategy — MSI for enterprise deployment (Group Policy), NSIS for consumer distribution (per-user install, custom UI)
- **OV Code Signing Certificate ($200-380/yr)**: Sufficient for SmartScreen reputation building post-March 2024 policy change; EV no longer provides instant trust
- **GitHub Actions with tauri-apps/tauri-action@v0**: Automates build, signing, and GitHub Release creation with updater JSON generation
- **Windows SDK (makeappx.exe)**: Manual MSIX packaging for Microsoft Store until Tauri supports native generation
- **GitHub Pages**: Zero-config static site hosting for landing page with dynamic download links via Releases API

**Critical version/compatibility notes:**
- Code signing certificates limited to 460 days maximum validity as of March 1, 2026 (annual renewal required)
- whisper.cpp requires LLVM/libclang for whisper-rs bindings; set `LIBCLANG_PATH` to VS-bundled LLVM path in CI
- WebView2 distribution mode: Recommend `downloadBootstrapper` (default) to keep installer <10MB; `offlineInstaller` adds 127MB for privacy-first positioning

### Expected Features

Microsoft Store submission requires four mandatory assets: privacy policy URL (must explain mic/clipboard access), age rating certification (self-serve questionnaire), app icons (300x300 + 192x192 color/outline), and minimum one screenshot at 1366x768+. Direct download distribution requires professionally designed icons (ICO with 16/32/256px), signed installers (both MSI and NSIS), and SHA-256 checksums for verification. Landing page should include hero section with download CTAs, privacy/trust messaging ("No cloud, no tracking"), visual demo (30-60 sec video), feature list, FAQ, and privacy policy link.

**Must have (table stakes):**
- Microsoft Store listing with privacy policy, age rating, screenshots (4-8), and icons — expected discovery path for Windows users (Signal Desktop added Store in Jan 2026)
- Direct download installers (MSI + NSIS) with code signing — users expect download without Store account
- Professional icon suite (multiple sizes: 16x16, 32x32, 256x256, 300x300) — system tray, taskbar, Store logo
- Privacy policy webpage (publicly hosted) — REQUIRED by Store for mic/clipboard access, even if no data collected
- Landing page with download links — central hub for direct distribution

**Should have (competitive advantage):**
- Offline-first MSI bundle (WebView2 embedded, +127MB) — aligns with privacy-first positioning, zero internet dependency
- Dual installer options (MSI for enterprise, NSIS for consumer) — respects user/IT preferences
- Auto-update with user control (downloads silently, user chooses install time) — via tauri-plugin-updater
- "No telemetry" transparency badge on landing page and Store listing — privacy differentiator vs cloud dictation services
- PGP-signed releases (in addition to code signing) — advanced user verification layer

**Defer (v2+ or optional):**
- Portable/no-install ZIP option — niche use case, manual packaging effort
- Deterministic builds with published hashes — strong security signal but high complexity
- Custom domain for landing page — GitHub Pages sufficient for v1.1
- Demo video/trailer for Store listing — optional asset, significant production effort

### Architecture Approach

The build pipeline follows a two-stage architecture: GitHub Actions builds standard installers (MSI + NSIS) using tauri-action, then a post-build step manually packages one artifact into MSIX using Microsoft's Windows SDK tools. Code signing must occur BEFORE MSIX packaging — sign the .exe first, then package the signed binary, then submit unsigned MSIX to Store (Microsoft re-signs with their certificate). The CI/CD workflow requires multi-layer caching: npm dependencies via `actions/setup-node` with lockfile cache, and Rust artifacts via `swatinem/rust-cache@v2` to speed up whisper.cpp recompilation from 15 minutes to 3 minutes. Website deployment runs independently via GitHub Pages or Vercel, with dynamic download links fetched from GitHub Releases API (no hardcoded URLs).

**Major components:**

1. **GitHub Actions CI/CD Pipeline** — Orchestrates environment setup (Node, Rust, MSVC), dependency installation, whisper.cpp compilation, Tauri build, code signing, artifact upload; uses `windows-latest` runner with pre-installed MSVC/CMake/Windows SDK
2. **Code Signing Infrastructure** — Integrates via `tauri.conf.json` with certificate thumbprint or Azure Key Vault; requires timestamp server retry logic (3 attempts with fallback URLs) to handle 5-15% server downtime; timestamps ensure binaries remain valid after certificate expiration
3. **Dual Installer Generation** — Tauri bundlers produce both MSI (WiX, enterprise-friendly, Group Policy support) and NSIS (per-user install, custom UI, smaller size); NSIS recommended as primary for consumer users
4. **MSIX Manual Packaging** — Post-build script uses `makeappx.exe pack` + `signtool.exe sign` to create Store-ready package from signed .exe; requires `AppxManifest.xml` with publisher name (distinct from product name), capabilities (microphone), and WebView2 dependency declaration
5. **Static Website with API Integration** — GitHub Pages hosts landing page; JavaScript fetches latest release metadata from `https://api.github.com/repos/.../releases/latest` to populate download buttons with current version URLs (eliminates manual updates)

**Key integration points:**
- Code signing happens in Phase 4 (after Tauri build, before MSIX packaging) — must sign .exe, not MSIX
- Website deployment is independent of installer build — can update landing page without new release
- Version single source of truth: `tauri.conf.json` → GitHub tag → workflow → MSIX manifest

### Critical Pitfalls

Research identified 15 pitfalls across critical/moderate/minor severity. The top 5 that should shape phase structure:

1. **SmartScreen Reputation Cliff (Even with EV Certificates)** — Microsoft changed policy in March 2024; EV certificates no longer grant instant trust. Both OV and EV now require organic reputation building through download telemetry (2-4 weeks minimum, 1000-5000 downloads). **Prevention:** Soft launch to trusted users first (50-100 installs), submit binary to Microsoft manually via SmartScreen file submission form, budget reputation time into launch timeline, never change publisher name mid-stream (resets to zero). Choose OV certificate to save $100-200/year with identical reputation outcomes.

2. **Microsoft Store MSIX Packaging Not Supported by Tauri** — Tauri v2 only generates MSI/NSIS, not MSIX. GitHub issues #4818 and #8548 track feature requests but show no roadmap commitment. **Prevention:** Accept manual MSIX workflow using Windows SDK tools (`makeappx.exe`, `signtool.exe`) or defer Store submission until Tauri adds native support. Test MSIX creation early to surface issues. Don't assume Tauri bundler will "just work" for Store.

3. **Timestamp Server Failures Break Builds (No Retry Logic)** — Timestamp servers (Sectigo, DigiCert, GlobalSign) have 5-15% downtime/rate limits. Without timestamp, signed binaries expire when certificate expires (460 days max). **Prevention:** Implement retry logic (3 attempts with 10-second delays), use multiple fallback timestamp URLs (`timestamp.sectigo.com` → `timestamp.digicert.com` → `timestamp.globalsign.com`), always timestamp (critical for long-term validity), monitor server health in CI logs.

4. **whisper.cpp CI/CD Build Complexity (MSVC + LLVM + CMake)** — GitHub Actions builds fail with "LLVM not found" or CMake errors when whisper-rs compiles whisper.cpp from source. **Prevention:** Pin runner to `windows-2022` (not `windows-latest`), explicitly set `LIBCLANG_PATH` to VS-bundled LLVM (`C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\Llvm\x64\bin`), cache Rust artifacts with `swatinem/rust-cache@v2`, verify toolchain before build with validation script.

5. **Dual Distribution Version Drift (Store vs Direct Download)** — Microsoft Store reviews take 3-7 days; direct downloads release instantly. Versions diverge, doubling support burden. **Prevention:** Synchronized release process (submit to Store 5-7 days before planned release, hold direct download until Store approval), use 4-part version numbering (`1.2.3.0` for Store, `1.2.3.1+` for direct hotfixes), tag telemetry with distribution channel (`store` vs `direct`), document version strategy in README.

**Additional moderate pitfalls to track:**
- Certificate validity cliff (460-day max since March 2026, budget annual renewals)
- Microsoft Store runtime download policy violations (ML models downloaded to AppData may trigger sandbox restrictions)
- WebView2 distribution choice impacts installer size (Download Bootstrapper <10MB vs Offline Installer +127MB)
- GitHub Actions artifact storage quota exhaustion (reduce retention to 7 days, only upload tagged releases)
- Installer testing on dirty systems (always test on fresh Windows VMs, non-admin user accounts)

## Implications for Roadmap

Based on combined research, recommend 4-phase structure over 4-5 weeks. This order prioritizes direct distribution first (lower complexity, faster user feedback) and defers Microsoft Store to second wave (manual MSIX workflow, Store review delays).

### Phase 1: Code Signing & Local Build Infrastructure (Week 1)

**Rationale:** Code signing is the foundation for all distribution channels. Without signed binaries, SmartScreen warnings block 30-70% of users. Establishes certificate infrastructure, tests signing locally before CI/CD complexity. Reputation building starts as soon as first signed binary is downloaded, so earlier is better.

**Delivers:**
- OV code signing certificate purchased and installed (Sectigo $279/yr recommended for cost/credibility balance)
- `tauri.conf.json` configured with certificate thumbprint, timestamp URL, digest algorithm
- Locally built MSI + NSIS installers signed and verified (test with `signtool verify /pa <file>`)
- Certificate password documented in team password manager (prevent "lost password" blocker)

**Addresses (from FEATURES.md):**
- Code signing certificate (table stakes) — establishes publisher identity
- Signed installers (table stakes) — prevents SmartScreen warnings

**Avoids (from PITFALLS.md):**
- Certificate password lost (document in team password manager)
- Wrong certificate type chosen (OV sufficient post-March 2024 policy change)
- Unsigned installers shipped (30-70% user abandonment)

**Stack elements used:**
- OV certificate from Sectigo/DigiCert/SSL.com
- Windows SDK SignTool.exe
- Tauri bundler with signing config

### Phase 2: GitHub Actions CI/CD Pipeline (Week 2)

**Rationale:** Automates repeatable builds before adding distribution complexity. Validates whisper.cpp compilation on clean runner environment. Establishes artifact caching to reduce 15-minute builds to 3 minutes. Release automation (GitHub Releases + updater JSON) enables direct download distribution and future auto-update feature.

**Delivers:**
- `.github/workflows/release.yml` triggered by version tags (`v*.*.*`)
- Environment setup: Node (with npm cache), Rust (stable), MSVC (via `ilammy/msvc-dev-cmd`), LLVM path configuration
- Rust artifact caching via `swatinem/rust-cache@v2` for whisper.cpp (cache key includes Cargo.lock hash)
- Timestamp server retry logic (3 attempts, multiple fallback URLs)
- Code signing secrets stored in GitHub (base64-encoded PFX + password OR Azure Key Vault credentials)
- `tauri-apps/tauri-action@v0` creates GitHub Release with MSI + NSIS + `latest.json` + signatures
- Verification: Push `v1.1.0-alpha` tag, confirm release artifacts generated and signed

**Addresses (from FEATURES.md):**
- Automated builds and releases (table stakes for professional distribution)
- Auto-update foundation (generates `latest.json` for future tauri-plugin-updater)
- Dual installer options (MSI + NSIS simultaneously)

**Avoids (from PITFALLS.md):**
- whisper.cpp build failures (explicit LLVM path, pinned runner, validated toolchain)
- Timestamp server failures (retry logic with fallbacks)
- Artifact storage quota exhaustion (7-day retention for non-release builds)

**Stack elements used:**
- GitHub Actions with `windows-latest` runner
- tauri-apps/tauri-action@v0
- Rust cache, npm cache
- Multiple timestamp server URLs

**Research flag:** Standard CI/CD patterns well-documented; skip phase-specific research.

### Phase 3: Landing Page & Direct Download Distribution (Week 3)

**Rationale:** Direct downloads provide immediate user feedback and start SmartScreen reputation building without Store review delays. Landing page establishes brand presence and central download hub. Dynamic download links eliminate manual updates for each release. Privacy policy required before Store submission (can reuse from landing page).

**Delivers:**
- Static website created in `docs/` folder (or separate repo)
- Hero section: "Press a key. Speak. Text appears." + screenshot/demo + "Download for Windows" CTA
- Privacy policy page (explains mic/clipboard access, local processing, no cloud/tracking) hosted at stable URL
- Download section with dynamic links (JavaScript fetches `https://api.github.com/repos/.../releases/latest`, populates MSI + NSIS URLs)
- Trust/privacy messaging: "Your voice never leaves your computer" + open source badge + GitHub link
- FAQ section: "Is it free?", "Offline?", "Privacy?", "Uninstall?"
- Footer with privacy policy link, GitHub link, license info (MIT/Apache)
- GitHub Pages enabled (Settings → Pages → Source: `main` branch, `/docs` folder)
- Site live at `https://yourusername.github.io/scribe/`

**Addresses (from FEATURES.md):**
- Landing page with download links (table stakes)
- Privacy policy webpage (REQUIRED for Store, even if no data collected)
- Direct download installers (table stakes)
- Trust/privacy messaging (competitive differentiator — "No cloud, no tracking")
- Professional presentation (hero, features, FAQ, trust signals)

**Avoids (from PITFALLS.md):**
- Missing privacy policy blocks Store submission (created now, reused in Phase 4)
- Hardcoded download URLs (dynamic API fetching, no manual updates)
- Feature-focused messaging instead of problem/solution (lead with user pain point)

**Stack elements used:**
- GitHub Pages (zero-config hosting)
- GitHub Releases API (dynamic download links)
- Static HTML/CSS/JS (no build step needed)

**Research flag:** Landing page design is generic; no phase-specific research needed. Use examples from FEATURES.md sources.

### Phase 4: Microsoft Store Submission (Week 4-5)

**Rationale:** Store submission comes after direct distribution to allow parallel reputation building and user feedback. Manual MSIX packaging requires Windows SDK tooling (Tauri doesn't support natively). Store review takes 3-7 days, so start submission 1 week before planned simultaneous release. Privacy policy from Phase 3 satisfies Store requirement.

**Delivers:**
- Microsoft Partner Center account created ($19 one-time fee for individual developers)
- App name reserved: "Scribe" (verify availability)
- Professional icons created: 300x300 color + outline (192x192 safe area), 192x192 for Store tile
- Screenshots captured: 4-8 images at 1366x768+ (overlay recording, history UI, settings, VU meter)
- Age rating certification completed (self-serve questionnaire, likely "Everyone")
- Store listing configured: short description, long description (HTML), category (Productivity), search terms ("voice typing, dictation, speech to text")
- Privacy policy URL linked (from Phase 3 landing page)
- MSIX packaging script created:
  - Build signed .exe via GitHub Actions
  - Create `AppxManifest.xml` with publisher name (distinct from "Scribe"), capabilities (microphone), WebView2 dependency
  - Use `makeappx.exe pack /d <folder> /p Scribe.msix`
  - Submit unsigned MSIX to Partner Center (Microsoft re-signs)
- Submission for certification (24-48 hours typical, budget 1 week for potential revisions)
- Monitor review feedback, address policy violations if flagged

**Addresses (from FEATURES.md):**
- Microsoft Store listing (table stakes)
- App icons (multiple sizes: 300x300, 192x192, ICO with 16/32/256) (table stakes)
- Screenshots (minimum 1, recommended 4-8) (table stakes)
- Privacy policy URL (REQUIRED by Store) (completed in Phase 3)
- Age rating (table stakes)

**Avoids (from PITFALLS.md):**
- Tauri MSIX not supported (use manual Windows SDK workflow)
- Missing privacy policy blocks submission (already created)
- Runtime model downloads violate Store policy (test early submission, transparent UI for downloads >100MB)
- Signing MSIX before Store submission (sign .exe first, submit unsigned MSIX)
- Version drift between Store and direct (synchronized release process, submit 1 week early)

**Stack elements used:**
- Windows SDK (makeappx.exe, signtool.exe)
- Microsoft Partner Center
- AppxManifest.xml (manual creation)

**Research flag:** MSIX packaging is non-standard for Tauri; may need troubleshooting during implementation. Test early submission to surface policy issues.

### Phase Ordering Rationale

This order follows dependency analysis and risk mitigation:

1. **Code signing first** because all distribution channels require signed binaries, and SmartScreen reputation building benefits from early start (2-4 weeks to accumulate downloads).

2. **CI/CD before distribution** because manual releases don't scale, and automating whisper.cpp builds on clean runner validates reproducibility before users encounter issues.

3. **Landing page before Store** because direct distribution provides faster user feedback (no 3-7 day review), starts reputation building sooner, and validates demand before Store investment. Privacy policy created for landing page satisfies Store requirement.

4. **Store last** because manual MSIX workflow adds complexity, Store review introduces 1-week delay, and synchronized release process requires direct distribution already established. Store is table stakes but not time-critical (many desktop apps launch direct-download-first).

**How this avoids pitfalls:**
- Soft launch via direct downloads (Phase 3) builds SmartScreen reputation before wide distribution
- Timestamp retry logic in CI/CD (Phase 2) prevents 5-15% build failures
- Privacy policy created in Phase 3 prevents Store submission blocker in Phase 4
- Version drift avoided via synchronized release (submit Store 1 week before direct release)

**Dependency chain:**
- Phase 2 depends on Phase 1 (certificate must exist to configure CI/CD signing)
- Phase 3 can run parallel to Phase 2 (landing page doesn't need automated releases)
- Phase 4 depends on Phase 3 (privacy policy URL required) and Phase 2 (MSIX packaging uses signed .exe from CI/CD)

### Research Flags

**Phases needing deeper research during planning:**
- **Phase 4 (Microsoft Store):** MSIX manifest creation for Tauri apps is underdocumented (Tauri doesn't provide template). Reference WSL-UI blog post or Microsoft MSIX samples. Test early submission to surface policy violations (runtime model downloads, AppContainer sandbox restrictions).

**Phases with standard patterns (skip research-phase):**
- **Phase 1 (Code Signing):** Well-documented in Tauri v2 docs + certificate provider documentation; standard workflow.
- **Phase 2 (CI/CD):** GitHub Actions + tauri-action is canonical approach; extensive community examples.
- **Phase 3 (Landing Page):** Static site hosting is straightforward; use examples from FEATURES.md sources.

**Open questions requiring user decision before requirements:**
1. **WebView2 distribution mode:** Download Bootstrapper (default, <10MB installer, requires internet) vs Offline Installer (+127MB, full privacy/offline alignment)? Recommendation: Offline Installer aligns with "local-first" positioning despite size increase.
2. **Custom domain for landing page:** GitHub Pages (`yourusername.github.io/scribe`) vs custom domain (`scribe-app.com`)? Recommendation: Defer custom domain to v1.2; GitHub Pages sufficient for v1.1.
3. **Certificate provider:** Sectigo ($279/yr OV) vs DigiCert ($380/yr OV) vs SSL.com ($65-249/yr OV)? Recommendation: Sectigo for balance of cost and enterprise credibility.
4. **Store submission timing:** Launch direct downloads first, then submit to Store 2-4 weeks later (allows reputation building + user feedback)? Or simultaneous release (higher risk, more complex coordination)? Recommendation: Stagger by 1 week (submit Store early, hold direct download until approval).

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Tauri v2 official docs extensively cover bundling, signing, CI/CD; WebSearch verified certificate pricing and SmartScreen policy changes |
| Features | HIGH | Microsoft Store requirements from official Learn documentation (Jan 2025); landing page best practices verified across multiple sources |
| Architecture | MEDIUM-HIGH | Build pipeline well-documented; MSIX manual workflow has fewer examples (WSL-UI blog post is primary reference) |
| Pitfalls | MEDIUM | SmartScreen reputation and timestamp failures verified across multiple sources; whisper.cpp CI/CD needs validation with actual build |

**Overall confidence:** HIGH for Phases 1-3, MEDIUM for Phase 4 (MSIX workflow less battle-tested)

### Gaps to Address

Research identified 4 areas requiring validation during implementation:

1. **MSIX `AppxManifest.xml` template for Tauri apps:** No official Tauri → MSIX manifest documented. Must reference WSL-UI blog post or create from Microsoft MSIX samples. **Mitigation:** Test MSIX creation in Phase 4 early; allocate buffer for troubleshooting.

2. **SmartScreen reputation threshold specifics:** Microsoft doesn't publish exact download count/timeframe for reputation building (estimates: 1000-5000 downloads over 2-4 weeks). **Mitigation:** Track SmartScreen warning frequency via user reports, focus on code signing (required) and soft launch (early adopters).

3. **Microsoft Store policy for runtime model downloads:** Unclear if 40MB-1.5GB ML model downloads to AppData violate "incomplete package" policy. **Mitigation:** Early test submission in Phase 4, transparent UI for downloads (show size, purpose, user consent), consider Windows ML Model Catalog APIs.

4. **whisper.cpp LLVM version compatibility on GitHub Actions:** Documentation doesn't specify minimum LLVM version for whisper-rs. **Mitigation:** Test CI workflow early in Phase 2; if VS-bundled LLVM fails, install standalone LLVM via chocolatey or `setup-cpp` action.

**NOT research gaps but implementation decisions:**
- WebView2 distribution mode choice (affects installer size, offline capability)
- Certificate provider selection (cost vs credibility tradeoff)
- Store vs direct distribution launch timing (simultaneous vs staggered)

## Sources

### Primary (HIGH confidence)

**Tauri Official Documentation:**
- [Tauri v2 Windows Installer](https://v2.tauri.app/distribute/windows-installer/)
- [Tauri v2 Code Signing](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri v2 Microsoft Store](https://v2.tauri.app/distribute/microsoft-store/)
- [Tauri v2 GitHub Actions](https://v2.tauri.app/distribute/pipelines/github/)
- [Tauri v2 Updater Plugin](https://v2.tauri.app/plugin/updater/)

**Microsoft Official Documentation:**
- [App screenshots, images, and trailers for MSIX](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/screenshots-and-images)
- [Add and edit Store listing info](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/add-and-edit-store-listing-info)
- [Microsoft Store Policies 7.19](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies)
- [MSIX Packaging with makeappx](https://learn.microsoft.com/en-us/windows/msix/package/create-app-package-with-makeappx-tool)
- [Microsoft SmartScreen Application Reputation](https://learn.microsoft.com/en-us/archive/blogs/ie/smartscreen-application-reputation-building-reputation)

**GitHub Actions Official:**
- [tauri-apps/tauri-action Repository](https://github.com/tauri-apps/tauri-action)
- [GitHub Actions Secrets Management](https://docs.github.com/en/actions/security-guides/encrypted-secrets)

### Secondary (MEDIUM confidence)

**Code Signing & Certificates:**
- [DigiCert: MS SmartScreen and Application Reputation](https://www.digicert.com/blog/ms-smartscreen-application-reputation)
- [Sectigo: MS SmartScreen FAQ](https://support.sectigo.com/PS_KnowledgeDetailPageFaq?Id=kA01N000000zFJx)
- [Microsoft Q&A: Reputation with OV certificates](https://learn.microsoft.com/en-us/answers/questions/417016/reputation-with-ov-certificates-and-are-ev-certifi)
- [SSL2Buy: Code Signing Certificate Validity Reduced to 460 Days](https://www.ssl2buy.com/wiki/code-signing-certificate-validity-reduced-to-460-days)
- [SSL Insights: Best Code Signing Certificate Providers 2026](https://sslinsights.com/best-code-signing-certificate-providers/)

**MSIX & Store Submission:**
- [Building WSL-UI: The Microsoft Store Journey](https://medium.com/@ian.packard/building-wsl-ui-the-microsoft-store-journey-b808e61cb167) — Real-world Tauri MSIX workflow
- [Tauri GitHub Issue #8548: Add MSIX generation](https://github.com/tauri-apps/tauri/issues/8548)
- [Windows ML Model Catalog Overview](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/model-catalog/overview)

**CI/CD & Build Infrastructure:**
- [whisper-rs Build Documentation](https://github.com/tazz4843/whisper-rs/blob/master/BUILDING.md)
- [whisper.cpp Windows Build Discussion](https://github.com/ggml-org/whisper.cpp/discussions/85)
- [GitHub Actions Artifact Storage Limits](https://medium.com/@aayushpaigwar/understanding-github-actions-artifact-storage-limits-and-how-to-manage-them-a577939f1c57)

**Landing Page & Distribution:**
- [GitHub Pages vs Cloudflare Pages Comparison](https://www.freetiers.com/blog/github-pages-vs-cloudflare-pages-comparison)
- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
- [15 Best App Landing Page Examples 2026](https://www.designrush.com/best-designs/apps/trends/app-landing-pages)

### Tertiary (LOW confidence, needs validation)

**Installer Best Practices:**
- [MSI vs EXE: Key differences | PDQ](https://www.pdq.com/blog/msi-vs-exe-the-battle-of-the-installers/)
- [EXE or MSI Installer | Advanced Installer](https://www.advancedinstaller.com/exe-vs-msi-installer.html)

**Privacy-First Positioning:**
- [Signal Desktop arrives in Microsoft Store](https://windowsforum.com/threads/signal-desktop-arrives-in-microsoft-store-privacy-first-windows-distribution.400561/)
- [Offline-First Apps: Key Use Cases 2026](https://www.octalsoftware.com/blog/offline-first-apps)

---

**Research completed:** 2026-02-16
**Ready for roadmap:** Yes
**Next step:** Create detailed requirements for each phase in REQUIREMENTS.md
