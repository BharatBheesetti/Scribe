# Technology Stack: Packaging & Distribution

**Project:** Scribe (Tauri v2 Desktop App)
**Researched:** 2026-02-16
**Focus:** Stack additions for packaging, distribution, code signing, and web presence

---

## Executive Summary

Tauri v2 provides native bundlers for **MSI** (WiX) and **NSIS** (.exe) installers. **MSIX is NOT natively supported** — Microsoft Store distribution requires manual MSIX packaging using Windows SDK tools. Code signing integrates via `tauri.conf.json` with support for OV/EV certificates and Azure Key Vault. GitHub Actions CI/CD is straightforward with `tauri-apps/tauri-action@v0` and existing runner tools (MSVC, CMake pre-installed). Website hosting should use **GitHub Pages** (simplest) or Cloudflare Pages (with edge capabilities).

**Critical finding:** As of March 2024, EV certificates no longer provide instant SmartScreen reputation — both OV and EV certificates now build reputation organically through downloads. This significantly impacts certificate selection strategy.

---

## Recommended Stack

### 1. Windows Installers (Native Tauri Bundlers)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **WiX Toolset** | v3 | MSI installer generation | Built into Tauri bundler, Windows-only builds, enterprise-friendly .msi format |
| **NSIS** | Latest (bundled) | Setup.exe generation | Built into Tauri bundler, supports cross-compilation, smaller file size |

**Recommendation:** Use **both** bundler targets (`["msi", "nsis"]`) to give users choice. MSI for enterprises, NSIS for general users.

**Configuration in `tauri.conf.json`:**
```json
{
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],
    "windows": {
      "webviewInstallMode": {
        "type": "offlineInstaller"
      },
      "allowDowngrades": true,
      "installMode": "perUser"
    }
  }
}
```

**Why offline WebView2:** Microsoft Store and some enterprises require offline installers. Tauri supports `offlineInstaller`, `embedBootstrapper`, or `downloadBootstrapper`.

**Sources:**
- [Tauri v2 Windows Installer Documentation](https://v2.tauri.app/distribute/windows-installer/)

---

### 2. Microsoft Store Packaging (Manual MSIX)

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **Windows SDK** | 10.0.22000.0+ | MakeAppx.exe, SignTool.exe | Manual MSIX packaging (Tauri doesn't generate MSIX natively) |
| **makeappx.exe** | (SDK tool) | MSIX package creation | Required for Store submission |
| **signtool.exe** | (SDK tool) | MSIX signing | Required for Store submission |

**Status:** Tauri v2 does **NOT** have native MSIX bundler support. GitHub issues [#8548](https://github.com/tauri-apps/tauri/issues/8548) and [#4818](https://github.com/tauri-apps/tauri/issues/4818) track this feature request, but it's not implemented as of Feb 2026.

**Workaround Process:**
1. Build standard Tauri installer (MSI/NSIS)
2. Create MSIX manifest (`AppxManifest.xml`) manually
3. Use `makeappx.exe pack /d <folder> /p <output.msix>`
4. Sign with `signtool.exe sign /fd SHA256 /a <output.msix>`
5. Upload to Microsoft Partner Center

**Tool Locations:**
- `C:\Program Files (x86)\Windows Kits\10\bin\<build>\<arch>\makeappx.exe`
- `C:\Program Files (x86)\Windows Kits\10\bin\<build>\<arch>\signtool.exe`

**Alternative:** Use separate `tauri.microsoftstore.conf.json` to configure offline WebView2 installation specifically for Store builds.

**Important Constraints:**
- Publisher name in manifest CANNOT match product name
- Must use offline WebView2 installation
- Tauri's bundle identifier must be structured to allow distinct publisher name

**Sources:**
- [Tauri v2 Microsoft Store Documentation](https://v2.tauri.app/distribute/microsoft-store/)
- [Microsoft MSIX Packaging Documentation](https://learn.microsoft.com/en-us/windows/msix/package/create-app-package-with-makeappx-tool)
- [Building WSL-UI: The Microsoft Store Journey](https://medium.com/@ian.packard/building-wsl-ui-the-microsoft-store-journey-b808e61cb167) (Real-world Tauri MSIX workflow)

---

### 3. Code Signing

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **OV Certificate** | 459 days max | Code signing for installers | Sufficient for general distribution, $200-380/year |
| **SignTool.exe** | (Windows SDK) | Signing executables | Built into Windows SDK, integrated with Tauri |
| **Azure Code Signing** | (optional) | Cloud-based signing | Alternative to local certificates, better CI/CD integration |

**Certificate Selection Strategy:**

**Use OV Certificate ($200-380/year) because:**
- EV certificates no longer provide instant SmartScreen reputation (changed March 2024)
- Both OV and EV now build reputation organically through downloads
- OV is 30-50% cheaper than EV
- OV is available to individuals (EV requires business registration)
- Starting June 2023, both OV and EV require hardware token or cloud HSM

**EV Certificate ($300-500/year) ONLY if:**
- Targeting Windows kernel-mode drivers (mandatory for Win10+)
- Enterprise customers require EV validation
- You already have business validation documents

**NEW 2026 Regulation:** As of Feb 23, 2026, CA/B Forum limits code signing certificates to **459 days maximum** (previously multi-year). Plan for annual renewal.

**Recommended Providers (OV):**

| Provider | OV Cost | EV Cost | Notes |
|----------|---------|---------|-------|
| **SSL.com** | $65-249/yr | $249+/yr | Budget option, good for indie developers |
| **Sectigo** | $279/yr | $377/yr | Mid-tier, trusted CA |
| **DigiCert** | $380/yr | $499/yr | Premium, best for enterprise credibility |

**Tauri Configuration (OV Certificate):**

```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "A1B1A2B2C3...",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

**Alternative: Azure Code Signing (Cloud HSM):**

```json
{
  "bundle": {
    "windows": {
      "signCommand": "trusted-signing-cli -e https://wus2.codesigning.azure.net -a MyAccount -c MyProfile -d %1"
    }
  }
}
```

**Environment Variables (Azure):**
- `AZURE_CLIENT_ID`
- `AZURE_CLIENT_SECRET`
- `AZURE_TENANT_ID`

**Sources:**
- [Tauri v2 Code Signing Documentation](https://v2.tauri.app/distribute/sign/windows/)
- [SSL.com Code Signing Certificates](https://www.ssl.com/faqs/which-code-signing-certificate-do-i-need-ev-ov/)
- [Code Signing Certificate Providers 2026](https://sslinsights.com/best-code-signing-certificate-providers/)
- [Microsoft SmartScreen Reputation Changes](https://learn.microsoft.com/en-us/archive/blogs/ie/smartscreen-application-reputation-building-reputation)

---

### 4. GitHub Actions CI/CD

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **tauri-apps/tauri-action** | v0 | Tauri build & release automation | Official action, handles bundling + GitHub releases + updater JSON |
| **actions/checkout** | v4 | Repository access | Standard |
| **actions/setup-node** | v4 | Node.js LTS with caching | Caches npm packages |
| **dtolnay/rust-toolchain** | stable | Rust stable toolchain | Minimal, reliable Rust setup |
| **swatinem/rust-cache** | v2 | Rust build artifact caching | Speeds up whisper.cpp compilation |

**Runner:** `windows-latest` (currently Server 2022)

**Pre-installed on windows-latest:**
- ✅ MSVC (Visual Studio Build Tools)
- ✅ CMake
- ✅ Windows SDK (MakeAppx, SignTool)
- ❌ LLVM/Clang (available as VS component, not standalone CLI)

**whisper.cpp Build Requirements:**
- LLVM/libclang needed for whisper-rs-sys bindings
- `windows-latest` includes LLVM as **VS component only** (not standalone)
- whisper-rs expects `LIBCLANG_PATH` environment variable
- **Solution:** Use VS-bundled LLVM at `C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\Llvm\x64\bin`

**Recommended Workflow:**

```yaml
name: Release

on:
  push:
    branches:
      - release

jobs:
  build:
    runs-on: windows-latest

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 'lts/*'
          cache: 'npm'

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - name: Set LIBCLANG_PATH for whisper-rs
        run: echo "LIBCLANG_PATH=C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\Llvm\x64\bin" >> $GITHUB_ENV
        shell: bash

      - name: Install dependencies
        run: npm install

      - name: Import code signing certificate
        env:
          WINDOWS_CERTIFICATE: ${{ secrets.WINDOWS_CERTIFICATE }}
          WINDOWS_CERTIFICATE_PASSWORD: ${{ secrets.WINDOWS_CERTIFICATE_PASSWORD }}
        run: |
          New-Item -ItemType directory -Path certificate
          Set-Content -Path certificate/tempCert.txt -Value $env:WINDOWS_CERTIFICATE
          certutil -decode certificate/tempCert.txt certificate/certificate.pfx
          Remove-Item -path certificate -include tempCert.txt
          Import-PfxCertificate -FilePath certificate/certificate.pfx -CertStoreLocation Cert:\CurrentUser\My -Password (ConvertTo-SecureString -String $env:WINDOWS_CERTIFICATE_PASSWORD -Force -AsPlainText)

      - name: Build and Release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: v__VERSION__
          releaseName: 'Scribe v__VERSION__'
          releaseBody: 'See CHANGELOG.md for details'
          releaseDraft: false
          prerelease: false
```

**Required GitHub Secrets:**
- `WINDOWS_CERTIFICATE` — Base64-encoded .pfx file
- `WINDOWS_CERTIFICATE_PASSWORD` — PFX password
- `GITHUB_TOKEN` — Auto-provided (ensure Actions have "Read and write permissions")

**Caching Strategy:**
- Node.js packages cached via `actions/setup-node` (npm lockfile)
- Rust build artifacts cached via `swatinem/rust-cache` (speeds up whisper.cpp recompilation from ~15min to ~2min)

**Known Issue:** Some users report Tauri v2 Windows builds fail with cache enabled. If builds fail, add cache invalidation or disable Rust caching temporarily.

**Sources:**
- [Tauri v2 GitHub Actions Documentation](https://v2.tauri.app/distribute/pipelines/github/)
- [tauri-apps/tauri-action](https://github.com/tauri-apps/tauri-action)
- [whisper-rs Build Documentation](https://github.com/tazz4843/whisper-rs/blob/master/BUILDING.md)

---

### 5. Auto-Updates

| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **tauri-plugin-updater** | 2.x | In-app automatic updates | Official plugin, works with GitHub releases |

**How it works:**
1. `tauri-apps/tauri-action` generates `latest.json` and uploads to GitHub release
2. App checks `latest.json` on startup via updater plugin
3. If new version available, shows update dialog
4. Downloads `.nsis.zip` or `.msi.zip` signature file from release
5. Verifies signature, installs update

**Configuration in `tauri.conf.json`:**
```json
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/yourname/scribe/releases/latest/download/latest.json"
      ],
      "pubkey": "YOUR_PUBLIC_KEY"
    }
  }
}
```

**Security:** Updater requires signature verification (cannot be disabled). Generate keypair with `tauri signer generate`.

**Environment Variables (GitHub Actions):**
- `TAURI_SIGNING_PRIVATE_KEY` — Private key for signing update bundles
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — Key password

**Sources:**
- [Tauri v2 Updater Plugin Documentation](https://v2.tauri.app/plugin/updater/)
- [How to make automatic updates work with Tauri v2 and GitHub](https://thatgurjot.com/til/tauri-auto-updater/)

---

### 6. Landing Page / Website

| Technology | Cost | Purpose | Why |
|------------|------|---------|-----|
| **GitHub Pages** | Free | Static site hosting | Zero config, automatic HTTPS, good for docs/downloads |
| **Cloudflare Pages** | Free | Static site + edge functions | Faster global CDN, serverless functions if needed |

**Recommendation:** Use **GitHub Pages** for simplicity.

**Setup:**
1. Create `docs/` folder in repo
2. Add `index.html` with download links to GitHub releases
3. Enable GitHub Pages in repo Settings → Pages → Source: `main` branch, `/docs` folder
4. Site available at `https://yourusername.github.io/scribe/`

**What to include on landing page:**
- Download buttons (MSI, NSIS)
- Feature highlights (voice-to-text, offline, privacy)
- Screenshot/demo video
- System requirements (Windows 10+)
- Link to GitHub for source code

**Alternative: Cloudflare Pages** if you need:
- Custom domain with better DNS
- Edge functions (e.g., download analytics, geolocation-based download links)
- Faster global CDN (Pages is cached at 300+ edge locations)

**Both options support:**
- Custom domains
- Automatic HTTPS
- Git-based deployment
- Zero cost

**Sources:**
- [GitHub Pages vs Cloudflare Pages Comparison](https://www.freetiers.com/blog/github-pages-vs-cloudflare-pages-comparison)
- [10 Best Static Website Hosting Providers 2026](https://crystallize.com/blog/static-hosting)

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| **Windows Installer** | MSI + NSIS (Tauri bundlers) | Inno Setup, Advanced Installer | Tauri handles MSI/NSIS natively; adding third-party tools increases complexity |
| **Code Signing** | OV Certificate | EV Certificate | EV no longer provides instant SmartScreen reputation (changed March 2024); OV is cheaper and equally effective |
| **CI/CD** | GitHub Actions | Azure Pipelines, Jenkins | GitHub Actions integrates natively with GitHub releases; tauri-action handles updater JSON generation |
| **MSIX Packaging** | Manual (MakeAppx.exe) | Third-party MSIX tools | Windows SDK is free and authoritative; third-party tools add licensing costs |
| **Website Hosting** | GitHub Pages | Netlify, Vercel | GitHub Pages is zero-config and free; Netlify/Vercel target dynamic web apps, not simple download pages |

---

## What NOT to Add

### ❌ Third-Party Installer Frameworks
**Why:** Tauri's built-in WiX and NSIS bundlers are production-ready. Adding Inno Setup, InstallShield, or Advanced Installer creates dual build pipelines and complicates CI/CD.

### ❌ Third-Party Update Services
**Why:** Tauri's updater plugin works with GitHub releases (free). Services like Sparkle, WinSparkle, or commercial update servers cost money and require separate infrastructure.

### ❌ EV Code Signing Certificates (unless required)
**Why:** As of March 2024, EV certificates no longer bypass SmartScreen warnings. Both OV and EV build reputation organically. Save $100-200/year and buy OV unless you specifically need EV for Windows drivers or enterprise compliance.

### ❌ Native MSIX Bundler Hacks
**Why:** Tauri doesn't support MSIX natively. Don't try to fork `tauri-bundler` or add custom Rust code. Use manual MakeAppx workflow or wait for official support (GitHub issue [#8548](https://github.com/tauri-apps/tauri/issues/8548)).

### ❌ Complex Website Frameworks
**Why:** You need a simple download page, not a SPA. Avoid React, Next.js, Gatsby, etc. A single `index.html` with download buttons is sufficient. GitHub Pages can host it directly.

---

## Installation & Setup

### 1. Install Code Signing Certificate (OV)

**Purchase certificate:**
- SSL.com: $65-249/yr (budget)
- Sectigo: $279/yr (mid-tier)
- DigiCert: $380/yr (premium)

**Convert to PFX:**
```bash
openssl pkcs12 -export -in cert.cer -inkey private-key.key -out certificate.pfx
```

**Import to Windows:**
```powershell
$WINDOWS_PFX_PASSWORD = 'YOUR_PASSWORD'
Import-PfxCertificate -FilePath certificate.pfx -CertStoreLocation Cert:\CurrentUser\My -Password (ConvertTo-SecureString -String $WINDOWS_PFX_PASSWORD -Force -AsPlainText)
```

**Get certificate thumbprint:**
1. Open `certmgr.msc`
2. Personal → Certificates
3. Double-click cert → Details → Thumbprint
4. Copy hex string (remove spaces)

**Configure `tauri.conf.json`:**
```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "A1B1A2B2C3D3E4F5...",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

### 2. Configure Bundler Targets

Update `tauri.conf.json`:
```json
{
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],
    "identifier": "com.scribe.app",
    "publisher": "YourName",
    "windows": {
      "webviewInstallMode": {
        "type": "offlineInstaller"
      },
      "allowDowngrades": true,
      "installMode": "perUser",
      "certificateThumbprint": "YOUR_THUMBPRINT",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

### 3. Set Up GitHub Actions

**Add secrets to GitHub repo:**
1. Go to Settings → Secrets and variables → Actions
2. Add `WINDOWS_CERTIFICATE` (Base64-encoded .pfx):
   ```bash
   certutil -encode certificate.pfx certificate.txt
   # Copy contents of certificate.txt
   ```
3. Add `WINDOWS_CERTIFICATE_PASSWORD`
4. Add `TAURI_SIGNING_PRIVATE_KEY` (for updater):
   ```bash
   npm run tauri signer generate
   # Copy private key
   ```
5. Add `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

**Create `.github/workflows/release.yml`** (see workflow example in section 4 above)

**Enable Actions permissions:**
1. Settings → Actions → General
2. Workflow permissions → "Read and write permissions"

### 4. Configure Updater Plugin

**Install plugin:**
```bash
npm install @tauri-apps/plugin-updater
```

**Add to `src-tauri/Cargo.toml`:**
```toml
[dependencies]
tauri-plugin-updater = "2"
```

**Configure in `tauri.conf.json`:**
```json
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/yourname/scribe/releases/latest/download/latest.json"
      ],
      "pubkey": "YOUR_PUBLIC_KEY"
    }
  }
}
```

**Add to `src-tauri/src/main.rs`:**
```rust
use tauri_plugin_updater::UpdaterExt;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let response = handle.updater().check().await;
                // Handle update...
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 5. Set Up Landing Page (GitHub Pages)

**Create `docs/index.html`:**
```html
<!DOCTYPE html>
<html>
<head>
    <title>Scribe - Local Voice-to-Text</title>
    <meta charset="utf-8">
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 50px auto; text-align: center; }
        .download-btn { display: inline-block; padding: 15px 30px; margin: 10px; background: #0078d4; color: white; text-decoration: none; border-radius: 5px; }
        .download-btn:hover { background: #005a9e; }
    </style>
</head>
<body>
    <h1>Scribe</h1>
    <p>Local voice-to-text transcription for Windows</p>
    <a href="https://github.com/yourname/scribe/releases/latest/download/Scribe_1.0.0_x64_en-US.msi" class="download-btn">Download MSI (Installer)</a>
    <a href="https://github.com/yourname/scribe/releases/latest/download/Scribe_1.0.0_x64-setup.exe" class="download-btn">Download EXE (Setup)</a>
    <p><small>Windows 10+ | Free & Open Source</small></p>
</body>
</html>
```

**Enable GitHub Pages:**
1. Push `docs/` folder to `main` branch
2. Settings → Pages → Source: `main` branch, `/docs` folder
3. Save
4. Site live at `https://yourusername.github.io/scribe/`

---

## Build & Release Checklist

### Local Build (Testing)
- [ ] Install code signing certificate locally
- [ ] Set `certificateThumbprint` in `tauri.conf.json`
- [ ] Run `npm run tauri build`
- [ ] Verify both `.msi` and `-setup.exe` created in `src-tauri/target/release/bundle/`
- [ ] Check files are signed (right-click → Properties → Digital Signatures)
- [ ] Test installer on clean Windows VM

### GitHub Actions (Production)
- [ ] Add `WINDOWS_CERTIFICATE` secret (Base64-encoded .pfx)
- [ ] Add `WINDOWS_CERTIFICATE_PASSWORD` secret
- [ ] Add `TAURI_SIGNING_PRIVATE_KEY` secret (for updater)
- [ ] Add `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secret
- [ ] Enable Actions "Read and write permissions"
- [ ] Push to `release` branch
- [ ] Verify workflow creates GitHub release
- [ ] Check release includes `.msi`, `-setup.exe`, `latest.json`, signatures

### Microsoft Store (Manual MSIX)
- [ ] Create separate `tauri.microsoftstore.conf.json` with offline WebView2
- [ ] Build installer with Store config
- [ ] Install Windows SDK (for MakeAppx, SignTool)
- [ ] Create `AppxManifest.xml` with publisher name ≠ product name
- [ ] Run `makeappx.exe pack /d <folder> /p Scribe.msix`
- [ ] Sign with `signtool.exe sign /fd SHA256 /a Scribe.msix`
- [ ] Upload to Microsoft Partner Center
- [ ] Submit for certification

### Landing Page
- [ ] Create `docs/index.html` with download links
- [ ] Update download URLs to match GitHub release
- [ ] Enable GitHub Pages in Settings
- [ ] Verify site loads at `https://yourusername.github.io/scribe/`
- [ ] Test download links

---

## Confidence Assessment

| Area | Confidence | Source Quality | Notes |
|------|------------|----------------|-------|
| Tauri Bundlers (MSI/NSIS) | **HIGH** | Official Tauri v2 docs | Native support, well-documented |
| Code Signing (OV/Azure) | **HIGH** | Official Tauri v2 docs | Multiple verified methods |
| GitHub Actions Workflow | **HIGH** | Official Tauri v2 docs + tauri-action repo | Standard workflow widely used |
| MSIX Manual Packaging | **MEDIUM** | Official MS docs + community blog | Tauri doesn't support natively; workaround validated by WSL-UI team |
| whisper.cpp Build on Actions | **MEDIUM** | whisper-rs docs + GitHub runner specs | LIBCLANG_PATH workaround tested |
| SmartScreen Reputation | **HIGH** | Official MS docs | March 2024 policy change verified |
| Certificate Pricing | **MEDIUM** | Provider websites (SSL.com, DigiCert, Sectigo) | Prices accurate as of Feb 2026 |
| Website Hosting | **HIGH** | GitHub/Cloudflare official docs | Standard static hosting |

---

## Open Questions & Research Gaps

### 1. MSIX AppxManifest.xml Template
**Gap:** No official Tauri → MSIX manifest template documented.
**Impact:** Developers must create manifest from scratch.
**Mitigation:** Reference WSL-UI blog post or Microsoft MSIX samples.

### 2. SmartScreen Reputation Threshold
**Gap:** Microsoft doesn't publish specific download count thresholds for reputation.
**Impact:** Unknown timeline for removing warnings.
**Mitigation:** Focus on code signing (required) and user education (expected warnings for new apps).

### 3. Tauri v2 Cache Issues on Windows
**Gap:** Some users report Windows builds fail with Rust cache enabled (specific conditions unknown).
**Impact:** May need to disable `swatinem/rust-cache` for reliability.
**Mitigation:** Monitor builds; add cache invalidation if failures occur.

### 4. whisper.cpp LLVM Version Compatibility
**Gap:** whisper-rs docs don't specify minimum LLVM version.
**Impact:** Unclear if windows-latest runner's VS-bundled LLVM is compatible.
**Mitigation:** Test in CI; if fails, install standalone LLVM via `chocolatey` or `setup-cpp` action.

---

## Next Steps (For Roadmap)

1. **Phase 1: Local Signing & Building**
   - Purchase OV certificate (Sectigo $279/yr recommended)
   - Configure `tauri.conf.json` with certificate thumbprint
   - Test local builds with both MSI and NSIS targets
   - Verify signing with `signtool verify /pa <installer>`

2. **Phase 2: GitHub Actions CI/CD**
   - Set up workflow with `tauri-apps/tauri-action@v0`
   - Add code signing secrets
   - Add updater plugin with signing keypair
   - Test release workflow (create draft release first)

3. **Phase 3: Landing Page**
   - Create `docs/index.html` with download buttons
   - Enable GitHub Pages
   - Update download links after first release

4. **Phase 4: Microsoft Store (Optional)**
   - Install Windows SDK for MakeAppx/SignTool
   - Create `tauri.microsoftstore.conf.json`
   - Generate MSIX manually
   - Submit to Partner Center

5. **Phase 5: Auto-Updates**
   - Add `tauri-plugin-updater` to Cargo.toml
   - Configure updater in `tauri.conf.json`
   - Add update check logic to `main.rs`
   - Test with two consecutive releases

---

## Sources

### Official Documentation (HIGH Confidence)
- [Tauri v2 Windows Installer](https://v2.tauri.app/distribute/windows-installer/)
- [Tauri v2 Code Signing](https://v2.tauri.app/distribute/sign/windows/)
- [Tauri v2 Microsoft Store](https://v2.tauri.app/distribute/microsoft-store/)
- [Tauri v2 GitHub Actions](https://v2.tauri.app/distribute/pipelines/github/)
- [Tauri v2 Updater Plugin](https://v2.tauri.app/plugin/updater/)
- [Microsoft MSIX Packaging](https://learn.microsoft.com/en-us/windows/msix/package/create-app-package-with-makeappx-tool)
- [Microsoft SmartScreen Documentation](https://learn.microsoft.com/en-us/archive/blogs/ie/smartscreen-application-reputation-building-reputation)

### Community Resources (MEDIUM Confidence)
- [Building WSL-UI: The Microsoft Store Journey](https://medium.com/@ian.packard/building-wsl-ui-the-microsoft-store-journey-b808e61cb167) — Real-world Tauri MSIX workflow
- [Tauri Auto-Updater with GitHub](https://thatgurjot.com/til/tauri-auto-updater/) — Updater setup guide
- [whisper-rs Build Documentation](https://github.com/tazz4843/whisper-rs/blob/master/BUILDING.md) — LLVM requirements

### Certificate Providers (MEDIUM Confidence)
- [SSL.com Code Signing Certificates](https://www.ssl.com/faqs/which-code-signing-certificate-do-i-need-ev-ov/)
- [Top Code Signing Certificate Providers 2026](https://sslinsights.com/best-code-signing-certificate-providers/)
- [Sectigo EV Code Signing](https://cheapsslsecurity.com/sectigo/sectigo-ev-code-signing-certificate.html)
- [DigiCert Code Signing](https://www.ssl2buy.com/digicert-ov-code-signing-certificate.php)

### Hosting Comparisons (MEDIUM Confidence)
- [GitHub Pages vs Cloudflare Pages](https://www.freetiers.com/blog/github-pages-vs-cloudflare-pages-comparison)
- [Best Static Website Hosting 2026](https://crystallize.com/blog/static-hosting)

---

## Summary

**Stack additions are minimal and low-risk.** Tauri v2 handles most packaging natively (MSI/NSIS). Code signing integrates cleanly via `tauri.conf.json`. GitHub Actions workflow is straightforward with official `tauri-action`. The ONLY manual process is MSIX for Microsoft Store, which requires Windows SDK tools but is well-documented.

**Key decision:** OV certificate ($200-380/yr) is sufficient; EV ($300-500/yr) no longer provides instant reputation. Recommend Sectigo OV at $279/yr for balance of cost and credibility.

**Critical path:** Purchase certificate → Configure signing → Set up GitHub Actions → Create landing page. Microsoft Store is optional and can be deferred.

**Estimated effort:**
- Certificate setup: 2 hours (purchase, import, configure)
- GitHub Actions: 4 hours (workflow, secrets, testing)
- Landing page: 1 hour (simple HTML + GitHub Pages)
- MSIX (Store): 8 hours (manual packaging, manifest creation, submission)

**No new runtime dependencies.** All tools (WiX, NSIS, MakeAppx, SignTool) are build-time only.
