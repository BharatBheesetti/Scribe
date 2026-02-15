# Feature Landscape: Windows Packaging & Distribution

**Domain:** Desktop application packaging, distribution, and trust establishment
**Project:** Scribe (privacy-first voice-to-text desktop app)
**Researched:** 2026-02-16

---

## Table Stakes

Features users expect from professional Windows desktop application releases. Missing these = product feels incomplete or untrustworthy.

| Feature | Why Expected | Complexity | Effort | Notes |
|---------|--------------|------------|--------|-------|
| **Microsoft Store Listing** | Standard discovery and installation path for Windows apps; Signal Desktop added MS Store in Jan 2026 | Medium | 3-5 days | Requires Partner Center account, privacy policy, age rating, screenshots, icons, description |
| **Direct Download Installer** | Users expect ability to download directly without Store account | Low | 1-2 days | Already supported by Tauri bundler (MSI + NSIS) |
| **Code Signing Certificate** | Required to avoid SmartScreen warnings; establishes publisher identity | Medium | 1-2 days + $249-500/year for EV cert | EV provides immediate SmartScreen trust; OV requires reputation building |
| **Professional Icons** | Store logo (300x300), app icon (multiple sizes: 16x16, 32x32, 256x256), system tray icon | Low | 2-3 days | MS Store requires 192x192 color + outline icons; Windows needs ICO with multiple sizes |
| **Screenshots (4-8)** | MS Store requires minimum 1, recommends 4-8 at 1366x768+ | Low | 1 day | Can reuse across languages; max 10 screenshots, PNG format, <50MB each |
| **Privacy Policy Page** | REQUIRED by MS Store for apps accessing personal data (mic, clipboard); legal compliance | Low | 1 day | Must explain data collection, storage, usage; hosted publicly with URL |
| **Age Rating** | MS Store certification requirement | Low | 30 min | Self-certification via questionnaire |
| **Uninstall Experience** | Clean removal via Settings > Apps, no leftover files/registry entries | Low | 1-2 days | Tauri bundlers handle this; verify settings cleanup in AppData |
| **Install for Current User** | Default non-admin install to %LOCALAPPDATA%; critical for enterprise adoption | Low | Built-in | Tauri default; also support perMachine mode for admin installs |
| **Silent Install Support** | MSI installers support `/quiet` by default; enterprise deployment requirement | Low | Built-in | MSI provides this automatically; NSIS needs configuration |
| **Versioning in Filenames** | Installer named `Scribe-1.0.0-setup.exe` not just `setup.exe` | Low | Built-in | Tauri bundler includes version automatically |

---

## Differentiators

Features that set Scribe's packaging apart. Not expected, but create competitive advantage for a privacy-first product.

| Feature | Value Proposition | Complexity | Effort | Notes |
|---------|-------------------|------------|--------|-------|
| **Offline-First MSI Bundle** | WebView2 embedded (~127MB increase) = zero internet required, full privacy | Low | 1 day config | Tauri `webviewInstallMode: "offlineInstaller"` — critical for privacy positioning |
| **Local-First Messaging** | Landing page emphasizes: "No cloud. No accounts. No tracking. Everything runs locally on your machine." | Low | 2-3 days | Competitive advantage vs. cloud-based dictation services |
| **Portable/No-Install Option** | ZIP with portable executable for users who can't/won't install software | Medium | 2-3 days | Tauri doesn't bundle this by default; manual packaging needed |
| **Deterministic Builds** | GitHub Actions with build reproducibility, published hashes for verification | High | 3-5 days | Enables security researchers to verify binaries; strong trust signal |
| **MSI + NSIS Dual Release** | Provide both installer types: MSI for enterprises, NSIS for flexibility | Low | Built-in | Tauri supports both; just enable both bundle targets |
| **No Telemetry Badge** | Explicit "No Analytics, No Tracking" badge on landing page + Store listing | Low | 1 day | Privacy differentiator; transparency builds trust |
| **Open Source Transparency** | Link to GitHub source on Store listing and installer welcome screen | Low | 1 day | "Inspect the code yourself" trust signal |
| **Auto-Update with User Control** | Updates download but user chooses when to install (not forced) | Medium | 3-5 days | Tauri updater plugin; respect user autonomy (anti-feature: forced updates) |
| **Installer Minimalism** | Single-screen install: "Click Install. That's it." No multi-step wizard for simple app | Low | 2-3 days | NSIS template customization; UX differentiator vs bloated installers |
| **PGP-Signed Releases** | In addition to code signing, provide PGP signatures for advanced users | Low | 1 day | Extra verification layer; 1% of users will check, but signals serious security |

---

## Anti-Features

Features to explicitly NOT build. Common mistakes in desktop app packaging that hurt trust/UX.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Bundled Adware/Toolbars** | Destroys trust instantly; common with free installers | Never bundle third-party offers, period |
| **Forced Auto-Start** | Installer enables auto-start without asking | Make auto-start opt-in during onboarding OR installer |
| **Registry Bloat** | Writing unnecessary registry keys, not cleaning up on uninstall | Only write essential keys (settings, uninstall info); clean up everything on removal |
| **Desktop Shortcut Without Asking** | Installers that create desktop shortcuts by default annoy users | NSIS: checkbox for desktop shortcut (default: unchecked) |
| **System-Wide Install by Default** | Requiring admin privileges when app doesn't need them | Default to currentUser install; offer perMachine as option |
| **Telemetry Opt-Out** | Analytics enabled by default with opt-out buried in settings | For privacy-first app: NO analytics at all. This is a feature. |
| **Forced Updates** | Auto-installing updates without user consent | Notify user of update, let them choose when to install |
| **Installer Bloat** | 500MB installer for 50MB app (poor WebView2 bundling choice) | Use downloadBootstrapper for minimum size OR offlineInstaller for privacy; document the tradeoff |
| **Nag Screens** | "Rate us!" / "Upgrade to Pro!" popups in free app | Never interrupt workflow; passive upgrade CTAs only |
| **Unsigned Installers** | Saving $500/year on EV cert costs thousands in lost users who see SmartScreen warnings | Budget for EV code signing certificate ($249-500/year) as critical infrastructure |
| **MS Store Exclusive** | Forcing users into Store when direct download is expected | Offer both Store + direct download options |
| **Confusing Uninstall** | Hiding uninstall process, making it multi-step | Standard Windows uninstall via Settings > Apps; one-click removal |

---

## Landing Page Sections

What users expect from a professional desktop app website (based on 2026 landing page analysis).

### Essential Sections (Table Stakes)

1. **Hero Section**
   - Headline: "Press a key. Speak. Text appears."
   - Subheading: "Local voice-to-text for Windows. No cloud, no accounts, no subscriptions."
   - Screenshot/video of overlay in action
   - CTA: "Download for Windows" + "View on Microsoft Store"
   **Complexity:** Low | **Effort:** 1-2 days

2. **Value Proposition Section**
   - "Why Scribe?" — 3-4 key benefits with icons
   - Privacy-first (all processing local)
   - Free and open source
   - Simple (press hotkey, speak, done)
   - Offline-capable
   **Complexity:** Low | **Effort:** 1 day

3. **Visual Demo Section**
   - 30-60 second video showing: hotkey press → speak → text appears in any app
   - GIF/video loop more effective than static screenshots
   - "See it in action" heading
   **Complexity:** Medium | **Effort:** 2-3 days (video recording + editing)

4. **Features List**
   - Bullet points with icons: cursor-following overlay, VU meter, filler removal, custom hotkey, history with search, model selection
   - "What makes Scribe different" framing
   **Complexity:** Low | **Effort:** 1 day

5. **Trust & Privacy Section**
   - "Your voice never leaves your computer"
   - Explicit privacy assurances: no cloud, no tracking, no accounts
   - Open source badge + GitHub link
   - "How it works" — local whisper.cpp processing
   **Complexity:** Low | **Effort:** 1 day

6. **Download Section**
   - Multiple CTAs: "Download Installer (.exe)" + "Download via Microsoft Store" + "View on GitHub Releases"
   - System requirements (Windows 10/11, 4GB RAM recommended)
   - Latest version number displayed
   **Complexity:** Low | **Effort:** 1 day

7. **FAQ Section**
   - Common questions: "Is it really free?", "Does it work offline?", "What about privacy?", "Which languages?", "How do I uninstall?"
   **Complexity:** Low | **Effort:** 1 day

8. **Footer**
   - Privacy Policy link (required for MS Store)
   - GitHub link
   - Contact/support
   - License info (MIT/Apache)
   **Complexity:** Low | **Effort:** 30 min

### Optional But High-Value Sections

9. **Social Proof** (when available)
   - User testimonials
   - GitHub stars count
   - Download count
   - "Featured on..." if applicable
   **Complexity:** Low | **Effort:** Ongoing (as social proof accumulates)

10. **Comparison Table** (future)
   - Scribe vs. Windows built-in voice typing vs. Dragon NaturallySpeaking
   - Emphasize privacy, offline, free differentiators
   **Complexity:** Medium | **Effort:** 2-3 days

---

## Microsoft Store Listing Requirements

Specific requirements for MS Store certification based on official documentation.

### Required Assets

| Asset | Specification | Notes |
|-------|---------------|-------|
| **App Icon (Color)** | 192x192 PNG, logo fits in 120x120 safe area | Square, no transparency |
| **App Icon (Outline)** | 192x192 PNG, outline version | For light/dark theme compatibility |
| **Store Logo (1:1 App Tile)** | 300x300 PNG | STRONGLY RECOMMENDED; Store prioritizes this over package icon |
| **Screenshots (Desktop)** | Min 1, recommend 4-8; 1366x768 or larger; PNG <50MB | Can be up to 4K (3840x2160) |
| **Description (Short)** | Text describing app value | Displayed in search results |
| **Description (Long)** | Detailed description with HTML formatting | Up to specific character limit |
| **Privacy Policy URL** | Public URL to hosted privacy policy | REQUIRED if app accesses personal data (mic = personal data) |
| **Age Rating** | Self-certification questionnaire | Likely "Everyone" or "Everyone 10+" for Scribe |
| **Category** | Productivity > Utilities or Productivity > Office | Choose 1 primary category |
| **Search Terms** | Keywords for Store search | "voice typing, dictation, speech to text, voice to text, transcription" |

### Optional But Recommended Assets

| Asset | Specification | Value |
|-------|---------------|-------|
| **2:3 Poster Art** | 720x1080 or 1440x2160 PNG | STRONGLY RECOMMENDED for games; apps can skip |
| **Trailer Video** | MP4/MOV, 1920x1080, <2GB, 60 sec or less | Requires 16:9 Super Hero Art (1920x1080); shows at top of listing |
| **16:9 Super Hero Art** | 1920x1080 or 3840x2160 PNG | Used for featured placement; no text on image |

### Content Policies

- **Privacy Policy:** Must explain what data is collected (audio for transcription), how it's used (local processing only), how it's stored (not transmitted)
- **No Age-Gated Content:** Scribe doesn't fall under this
- **No Deceptive Functionality:** App must do what it says (voice-to-text ✓)
- **Clean Uninstall:** Must remove cleanly via Settings > Apps

**Complexity:** Medium (first-time setup), Low (maintenance)
**Effort:** 3-5 days for initial submission + review time (typically 24-48 hours)

---

## Code Signing & Trust Signals

What users see and what it means for adoption.

### Without Code Signing

**User Experience:**
- Windows Defender SmartScreen warning: "Windows protected your PC"
- "Unknown publisher" label
- User must click "More info" → "Run anyway"
- **Conversion impact:** 30-70% of users abandon download

**Why This Happens:**
- Unsigned executables have no verified publisher identity
- SmartScreen assumes unsigned = potentially malicious

### With Organization Validation (OV) Code Signing

**User Experience:**
- Installer shows verified publisher name
- SmartScreen warning STILL appears initially (until reputation builds)
- Reputation builds organically as users install and run the app
- After ~1000-5000 downloads with no malware reports, SmartScreen warnings decrease

**Cost:** ~$65-150/year
**Build Time:** Low (signing during build)
**Trust Time:** Weeks to months to build reputation

### With Extended Validation (EV) Code Signing

**User Experience:**
- Installer shows verified publisher name
- **Immediate SmartScreen trust** (traditionally bypassed warning)
- ⚠️ **2024 Change:** Some reports indicate EV certs may now also require reputation building (research contradictory)

**Cost:** $249-500/year
**Build Time:** Low (signing during build)
**Trust Time:** Immediate (or very fast)

**Recommendation for Scribe:** Start with **EV certificate** from DigiCert, Sectigo, or SSL.com. The $500/year cost is offset by:
- Near-zero user friction on install
- Professional publisher identity
- Faster adoption (fewer abandoned downloads)
- Required for serious distribution

### Trust Signals Users See (In Priority Order)

1. **Verified Publisher Name** (from code signing cert) — Top trust signal
2. **No SmartScreen Warning** (EV cert OR established reputation) — Critical for conversion
3. **Microsoft Store Badge** ("Available on Microsoft Store") — Implies MS reviewed it
4. **GitHub Stars / Download Count** — Social proof
5. **Open Source Badge** — Transparency signal (niche audience)
6. **Privacy Policy Link** — Shows professionalism
7. **PGP Signature** (for advanced users) — Extra verification layer

**Complexity:** Medium (cert acquisition, build pipeline integration)
**Effort:** 1-2 days setup + annual renewal

---

## GitHub Releases Best Practices

What users expect from desktop app GitHub releases (2026 standards).

### Required Artifacts

| Artifact | Format | Notes |
|----------|--------|-------|
| **Windows Installer (NSIS)** | `Scribe-1.0.0-setup.exe` | Signed with code signing cert |
| **Windows Installer (MSI)** | `Scribe-1.0.0.msi` | Signed; for enterprise deployment |
| **Source Code (Auto)** | `.zip` + `.tar.gz` | GitHub auto-generates these |
| **Checksums** | `SHA256SUMS.txt` | SHA-256 hashes for all binaries |

### Optional But Recommended Artifacts

| Artifact | Format | Value |
|----------|--------|-------|
| **Portable Build** | `Scribe-1.0.0-portable.zip` | No-install option for paranoid users |
| **PGP Signatures** | `.asc` files for each binary | Advanced user verification |
| **Release Notes** | Markdown in release description | See format below |

### Release Notes Format (Based on Keep a Changelog + Conventional Commits)

```markdown
## [1.0.0] - 2026-02-20

### Added
- Custom hotkey configuration (default: Ctrl+Shift+Space)
- Filler word removal (um, uh, like) with language detection
- Sound effects for recording start/stop
- History search with highlighting

### Fixed
- Microphone muting detection via IAudioEndpointVolume
- Overlay positioning on multi-monitor setups

### Changed
- Upgraded whisper.cpp to v1.5.0 for 15% faster transcription

### Security
- All binaries now signed with EV code signing certificate

**Full Changelog:** https://github.com/user/scribe/compare/v0.9.0...v1.0.0
**Download:** [Windows Installer (recommended)](link) | [MSI](link) | [Portable](link)
**Verify:** SHA256: `abc123...`
```

**Key Elements:**
- Semantic versioning (MAJOR.MINOR.PATCH)
- Categorized changes (Added, Fixed, Changed, Security, Deprecated, Removed)
- Links to full changelog (GitHub compare view)
- Download links for each artifact
- SHA-256 checksums for verification
- Date in ISO format (YYYY-MM-DD)

**Tools for Automation:**
- `release-please` (GitHub Action) — auto-generates releases from conventional commits
- `commit-and-tag-version` — local CLI tool for versioning + changelog
- Manual for first few releases, then automate

**Complexity:** Low (manual), Medium (automated)
**Effort:** 30-60 min per release (manual), 1 day setup (automated)

---

## Installer UX Expectations

What users expect from the Windows installer experience (MSI vs. NSIS).

### Default Installer Flow (NSIS — Recommended for Scribe)

**Expected Screens:**
1. **Welcome Screen**
   - "Welcome to Scribe Setup"
   - Version number displayed
   - "Click Install to begin"
   - Checkbox: "☐ Create desktop shortcut" (default: unchecked)
   - Checkbox: "☑ Start Scribe when Windows starts" (default: checked, since onboarding already handles this)

2. **License Screen** (if included)
   - MIT/Apache license text
   - "I accept the terms" checkbox
   - Skip this for open source — adds friction

3. **Installation Progress**
   - Progress bar with current file being copied
   - "Installing Scribe..." heading
   - Should complete in <30 seconds for ~150MB app

4. **Completion Screen**
   - "Setup has finished installing Scribe"
   - Checkbox: "☑ Launch Scribe" (default: checked)
   - "Finish" button

**Total Clicks:** 2-3 (Install → Finish)
**Total Time:** <60 seconds

**Anti-Patterns to Avoid:**
- Multi-step component selection (Scribe has no optional components)
- Custom install path picker (99% of users want default location)
- "Newsletter signup" or promotional screens
- Multiple confirmation dialogs

### MSI Installer Flow

**Expected Experience:**
- Windows standard installer UI (less customizable)
- Install for current user (default) OR system-wide (if admin)
- Silent install support: `msiexec /i Scribe-1.0.0.msi /quiet`
- Appears in Settings > Apps for uninstall
- Enterprise deployment via Group Policy

**Advantages over NSIS:**
- Standardized silent install (`/quiet`, `/qn`)
- Better for enterprise environments (GPO, Intune)
- Windows Installer service handles repair/rollback

**Disadvantages:**
- Less customizable UI
- Can only be built on Windows
- Slightly larger file size

**Recommendation:** Provide BOTH (Tauri makes this easy). NSIS for end users (better UX), MSI for enterprises.

### Custom Install Path (Low Priority)

**User Expectation:** 95% of users install to default location
**When Needed:** Users with D:/ drive for apps, portable installs
**Complexity:** Medium (NSIS template customization)
**Recommendation:** Skip for v1.0; add if users request

### WebView2 Installation Options (Critical Decision for Scribe)

Tauri offers 5 modes:

| Mode | Size Impact | Internet Required | Recommendation for Scribe |
|------|-------------|-------------------|---------------------------|
| `downloadBootstrapper` | +0 MB | Yes (downloads ~127MB) | ❌ Conflicts with "offline-first" positioning |
| `embedBootstrapper` | +1.8 MB | Yes (downloads ~127MB) | ❌ Still requires internet |
| `offlineInstaller` | +127 MB | No | ✅ **RECOMMENDED** — aligns with privacy/offline messaging |
| `fixedVersion` | +180 MB | No | ⚠️ Overkill; only if specific WebView2 version needed |
| `skip` | +0 MB | No (assumes WebView2 already installed) | ❌ Will fail on fresh Windows installs |

**Decision:** Use `offlineInstaller`. The 127MB increase is worth it for:
- True offline installation (matches privacy-first positioning)
- No internet required (critical for "local-first" messaging)
- No installation failures due to missing WebView2

**Tradeoff:** Installer size increases from ~30MB → ~160MB. Acceptable for desktop app in 2026.

---

## Feature Dependencies & Phase Recommendations

### Dependencies on Existing Scribe Features

| Packaging Feature | Depends On (Existing) | Notes |
|-------------------|-----------------------|-------|
| MS Store listing screenshots | Overlay, VU meter, history UI | Already have UI to screenshot ✓ |
| Privacy Policy content | No cloud processing, local storage architecture | Architecture already supports privacy claims ✓ |
| Uninstall cleanup | Settings stored in AppData | Verify settings.json removal on uninstall |
| Auto-start toggle in installer | Existing auto-start feature (F5) | Can sync installer checkbox with app setting |
| Landing page "How it works" | whisper.cpp local processing | Can explain technical architecture |

### New Dependencies (What We Need to Build)

1. **Privacy Policy Page** — Standalone webpage, hosted publicly (GitHub Pages or domain)
2. **Landing Page/Website** — Can be GitHub Pages, custom domain, or static site
3. **EV Code Signing Certificate** — Purchase + integrate into build pipeline
4. **MS Store Partner Center Account** — $19 one-time registration (individual) or $99/year (company)
5. **Icon Suite** — Professional icons in all required sizes (hire designer or use Figma kit)
6. **Screenshots** — Capture 4-8 high-quality screenshots at 1366x768+
7. **Demo Video** (optional) — 30-60 sec screen recording for landing page + MS Store trailer

### Recommended Phases

**Phase 1: Minimum Viable Distribution (MVP)** — 1-2 weeks
- EV code signing certificate (purchase + integrate)
- Privacy policy page (GitHub Pages)
- Professional icon suite (300x300, 192x192, ICO with 16/32/256)
- GitHub Releases with signed installers (NSIS + MSI)
- Basic landing page (GitHub Pages with download links)

**Phase 2: Microsoft Store** — 1 week (after Phase 1)
- Partner Center account
- 4-8 screenshots
- Store listing copy (short + long description)
- Age rating certification
- Submit for review
- **Blocker:** Requires privacy policy (Phase 1 dependency)

**Phase 3: Landing Page Polish** — 1-2 weeks
- Custom domain (optional)
- Demo video (30-60 sec)
- FAQ section
- "How it works" explainer
- Social proof section (when available)

**Phase 4: Advanced Distribution** — Ongoing
- Automated release notes (release-please)
- Portable build option
- PGP signatures
- Deterministic builds
- Auto-update implementation (Tauri updater plugin)

---

## Complexity & Effort Summary

### By Complexity Level

**Low Complexity (1-3 days each):**
- Direct download installer setup (Tauri bundler config)
- Icon creation (if using templates/Figma kit)
- Screenshots
- Privacy policy page
- Basic landing page
- GitHub Releases manual process
- Age rating certification
- Uninstall verification

**Medium Complexity (3-5 days each):**
- Microsoft Store submission (first time)
- EV code signing cert acquisition + integration
- Demo video recording + editing
- NSIS installer customization (desktop shortcut checkbox, etc.)
- Auto-update implementation
- Portable build packaging
- Automated release notes setup

**High Complexity (5+ days):**
- Deterministic builds
- Full landing page with custom design
- Comparison table research + creation

### Total Effort Estimates

**Minimum Viable Distribution (Phase 1):** 10-15 days
**Microsoft Store (Phase 2):** +5-7 days
**Landing Page Polish (Phase 3):** +7-10 days
**Advanced Features (Phase 4):** +10-15 days (spread over time)

**Total for Professional Release:** 25-35 days (5-7 weeks)

---

## Confidence Assessment

| Area | Confidence | Source Quality |
|------|------------|----------------|
| Microsoft Store Requirements | HIGH | Official Microsoft Learn documentation (Jan 2025) |
| Code Signing Impact | HIGH | Multiple authoritative sources + developer community consensus |
| Landing Page Patterns | MEDIUM | Design blogs + examples (verified 2026 trends) |
| Installer UX Expectations | MEDIUM | Industry best practices + Windows user norms |
| GitHub Releases Format | HIGH | Keep a Changelog standard + GitHub official docs |
| Tauri Bundler Capabilities | HIGH | Official Tauri v2 documentation |
| WebView2 Bundling Tradeoffs | HIGH | Tauri official docs + size measurements |
| Privacy-First Positioning | MEDIUM | Inferred from Signal, Obsidian, local-first app marketing analysis |

### Research Gaps (To Address in Implementation)

- **MS Store Review Time:** Documentation says "typically 24-48 hours" but need to budget for potential delays (up to 1 week)
- **EV Cert Reputation Building (2024 Change):** Conflicting reports on whether EV certs still bypass SmartScreen immediately; may need to test
- **Actual Installer Size with offlineInstaller:** Documentation says +127MB but need to verify final size
- **Custom NSIS Template Complexity:** May be easier/harder than estimated depending on Tauri's template customization hooks

---

## Sources

### Microsoft Official Documentation

- [App screenshots, images, and trailers for MSIX app - Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/screenshots-and-images)
- [Add and edit Store listing info for MSIX app - Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/add-and-edit-store-listing-info)
- [Microsoft Store Policies version 7.19 - Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies)
- [Design guidelines for Windows app icons - Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/design/style/iconography/app-icon-design)
- [Construct your Windows app's icon - Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/design/style/iconography/app-icon-construction)

### Tauri Documentation

- [Windows Installer | Tauri v2](https://v2.tauri.app/distribute/windows-installer/)

### Code Signing & Trust

- [Top 10 Best Code Signing Certificate Providers in 2026](https://sslinsights.com/best-code-signing-certificate-providers/)
- [How EV Code Signing Works - Bypass the Microsoft SmartScreen Warning](https://aboutssl.org/how-ev-code-signing-works/)
- [MS SmartScreen and Application Reputation - Sectigo Knowledge Base](https://support.sectigo.com/PS_KnowledgeDetailPageFaq?Id=kA01N000000zFJx)
- [Reputation with OV certificates and are EV certificates still the better option? - Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/417016/reputation-with-ov-certificates-and-are-ev-certifi)

### Installer Best Practices

- [MSI vs EXE: Key differences for software installs | PDQ](https://www.pdq.com/blog/msi-vs-exe-the-battle-of-the-installers/)
- [EXE or MSI Installer - Differences and Recommendations | Advanced Installer](https://www.advancedinstaller.com/exe-vs-msi-installer.html)
- [MSI vs EXE: Ultimate guide to Windows installers | Superops](https://superops.com/tech-hub/msi-vs-exe)
- [How to Install MSI for All Users - 2026 Best Practices](https://copyprogramming.com/howto/installing-msi-or-exe-to-all-users-profile-command-line)

### Landing Page Design

- [15 Best App Landing Page Examples (2026) | DesignRush](https://www.designrush.com/best-designs/apps/trends/app-landing-pages)
- [17 Full Length App Landing Page Examples To Copy [2026] | KlientBoost](https://www.klientboost.com/landing-pages/app-landing-page/)
- [9 Security Landing Pages to Help Make Your Page More Trustworthy | Instapage](https://instapage.com/blog/security-landing-pages)

### Privacy-First & Offline-First Positioning

- [Signal Desktop arrives in Microsoft Store - Windows Forum](https://windowsforum.com/threads/signal-desktop-arrives-in-microsoft-store-privacy-first-windows-distribution.400561/)
- [Offline-First Apps: Key Use Cases and Benefits in 2026 | Octal Software](https://www.octalsoftware.com/blog/offline-first-apps)
- [Why Offline First Apps Will Dominate In 2026 | Medium](https://medium.com/@tekwrites/why-offline-first-apps-are-dominating-2026-c76e5083d686)
- [The Rise of Local-First & Offline-First Apps | Medium](https://medium.com/@e.storefashion07/the-rise-of-local-first-offline-first-apps-redefining-digital-experiences-47a8d3f36484)

### GitHub Releases

- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
- [Automatically generated release notes - GitHub Docs](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes)
- [Creating GitHub Releases with Binary Artifacts - Zero to Hero](https://zerotohero.dev/inbox/github-releases/)
- [GitHub - release-it/release-it](https://github.com/release-it/release-it)

### Uninstall & App Management

- [Uninstall or remove apps and programs in Windows - Microsoft Support](https://support.microsoft.com/en-us/windows/uninstall-or-remove-apps-and-programs-in-windows-4b55f974-2cc6-2d2b-d092-5905080eaf98)
- [How to Completely Remove Apps and Programs on Windows | How-To Geek](https://www.howtogeek.com/how-to-completely-remove-apps-and-programs-on-windows/)
- [Windows 11 Store Library Uninstall - Windows Forum](https://windowsforum.com/threads/windows-11-store-library-uninstall-and-enterprise-policy-for-preinstalled-apps.391128/)

---

**Research Complete:** 2026-02-16
**Next Step:** Use this research to inform requirements definition for packaging/distribution milestone.
