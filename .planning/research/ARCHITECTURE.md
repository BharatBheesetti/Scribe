# Build & Release Pipeline Architecture

**Project:** Scribe
**Domain:** Windows Desktop App Distribution
**Researched:** 2026-02-16
**Overall Confidence:** MEDIUM-HIGH

## Executive Summary

The build/release pipeline for Scribe involves five distinct systems that must integrate: GitHub Actions CI/CD, code signing infrastructure, dual installer packaging (MSI + NSIS), MSIX creation for Microsoft Store, and static website deployment. The critical constraint is whisper.cpp compilation requiring a full C++ toolchain (MSVC, CMake, LLVM) on the GitHub Actions runner.

**Key architectural decision:** Use a two-stage build process - first build standard installers (MSI/NSIS) with GitHub Actions + tauri-action, then post-process one artifact to create MSIX using Microsoft's new winapp CLI tool (announced January 2026).

**Critical integration point:** Code signing must happen BEFORE MSIX packaging. Sign the .exe first, then package the signed binary into MSIX.

## Build Pipeline Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                      GitHub Actions Trigger                      │
│            (Push to main with tag: v*.*.* pattern)              │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 1: Environment Setup                    │
│                                                                   │
│  • Checkout code (actions/checkout@v4)                          │
│  • Setup Node.js (actions/setup-node@v4) with caching          │
│  • Setup Rust (dtolnay/rust-toolchain@stable)                  │
│  • Setup MSVC Dev Command (ilammy/msvc-dev-cmd@v1)             │
│  • Cache Rust artifacts (swatinem/rust-cache@v2)               │
│                                                                   │
│  Runner: windows-latest (includes MSVC, CMake)                  │
│  LLVM: Already pre-installed on windows-latest runner          │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Phase 2: Dependency Installation                 │
│                                                                   │
│  • npm install (frontend dependencies)                          │
│  • cargo build dependencies compile whisper-rs-sys             │
│    → Invokes CMake to compile whisper.cpp from source          │
│    → Requires: MSVC compiler, CMake, LLVM (for llvm-rc)        │
│                                                                   │
│  Artifact: whisper.cpp compiled as static library               │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Phase 3: Tauri Build                          │
│                                                                   │
│  • tauri-apps/tauri-action@v0 with inputs:                      │
│    - tagName: ${{ github.ref_name }}                            │
│    - releaseName: "Scribe v__VERSION__"                         │
│    - releaseBody: (from CHANGELOG or auto-generated)            │
│    - releaseDraft: false                                        │
│                                                                   │
│  Generates (in src-tauri/target/release/bundle/):               │
│    • msi/Scribe_1.0.0_x64.msi (WiX installer)                   │
│    • nsis/Scribe_1.0.0_x64-setup.exe (NSIS installer)          │
│    • Plain binary: scribe.exe (if uploadPlainBinary: true)     │
│                                                                   │
│  Files uploaded to GitHub Release automatically                  │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Phase 4: Code Signing                          │
│                                                                   │
│  Option A: Traditional Certificate (PFX file)                    │
│    • Decode PFX from base64 secret                              │
│    • Sign with signtool.exe via Tauri config                   │
│                                                                   │
│  Option B: Azure Code Signing (RECOMMENDED for 2026)            │
│    • Use Azure Artifact Signing service                         │
│    • Authenticate with Azure credentials from secrets           │
│    • Sign using AzureSignTool CLI                               │
│                                                                   │
│  Sign BEFORE MSIX packaging - sign the .exe directly            │
│                                                                   │
│  New requirement (2026): Max 460-day cert validity              │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Phase 5: MSIX Packaging                        │
│                                                                   │
│  Tool: Microsoft winapp CLI (announced Jan 2026)                │
│                                                                   │
│  Steps:                                                          │
│    1. winapp init (configure project metadata)                  │
│    2. winapp manifest (generate AppxManifest.xml)               │
│    3. winapp package (convert signed .exe → .msix)              │
│                                                                   │
│  Input: Signed scribe.exe + assets                              │
│  Output: Scribe_1.0.0_x64.msix (unsigned for Store)            │
│                                                                   │
│  Note: Microsoft Store re-signs MSIX with their cert            │
│        Submit UNSIGNED MSIX to Partner Center                    │
└────────────────────┬────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Phase 6: Artifact Distribution                  │
│                                                                   │
│  GitHub Release (automated by tauri-action):                    │
│    • Scribe_1.0.0_x64.msi (for direct download)                │
│    • Scribe_1.0.0_x64-setup.exe (for direct download)          │
│    • latest.json (updater manifest)                             │
│                                                                   │
│  Manual Upload to Microsoft Store Partner Center:               │
│    • Scribe_1.0.0_x64.msix (unsigned)                          │
│                                                                   │
│  Website Deploy (automated):                                     │
│    • Push static site to GitHub Pages / Vercel                  │
│    • Update download links to GitHub Release URLs               │
└─────────────────────────────────────────────────────────────────┘
```

## Integration with Existing Tauri Build

### Current Build System

**Existing flow:**
1. `npm run tauri build` invokes Tauri CLI
2. Tauri CLI runs `cargo build --release`
3. Cargo compiles whisper-rs dependency
4. whisper-rs-sys build script invokes CMake to compile whisper.cpp
5. Tauri bundles frontend + Rust binary into installers

**GitHub Actions replicates this:**
- Must have same toolchain as local dev environment
- MSVC from `ilammy/msvc-dev-cmd` action
- CMake already on windows-latest runner
- LLVM already on windows-latest runner (for llvm-rc)

### Modified tauri.conf.json

```json
{
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],  // Build both installer types
    "icon": ["icons/icon.png"],
    "windows": {
      "certificateThumbprint": null,  // Set via environment variable in CI
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.comodoca.com",
      "webviewInstallMode": {
        "type": "downloadBootstrapper"  // Download WebView2 at install time
      },
      "wix": {
        "language": "en-US"
      },
      "nsis": {
        "installMode": "currentUser",  // No admin required
        "displayLanguageSelector": false
      }
    }
  }
}
```

### Signing Integration

**Two approaches:**

#### Option A: Traditional PFX Certificate
```yaml
# In GitHub Actions workflow
- name: Sign executables
  env:
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.CERTIFICATE_PFX_BASE64 }}
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.CERTIFICATE_PASSWORD }}
  run: |
    # Decode certificate
    echo $TAURI_SIGNING_PRIVATE_KEY | base64 -d > cert.pfx

    # Configure Tauri to sign
    # Set WINDOWS_CERTIFICATE_THUMBPRINT env var
```

#### Option B: Azure Code Signing (Recommended)
```yaml
# In GitHub Actions workflow
- name: Install AzureSignTool
  run: dotnet tool install --global AzureSignTool

- name: Sign with Azure
  env:
    AZURE_KEY_VAULT_URL: ${{ secrets.AZURE_KEY_VAULT_URL }}
    AZURE_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}
    AZURE_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}
    AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}
  run: |
    azuresigntool sign \
      -kvu "$AZURE_KEY_VAULT_URL" \
      -kvi "$AZURE_CLIENT_ID" \
      -kvs "$AZURE_CLIENT_SECRET" \
      -kvt "$AZURE_TENANT_ID" \
      -tr http://timestamp.digicert.com \
      -td sha256 \
      src-tauri/target/release/scribe.exe
```

**Why Azure Code Signing?**
- Announced January 2026 as new standard approach
- More secure (private key never leaves Azure)
- Supports new 460-day maximum certificate validity requirement
- Integrates with GitHub Actions via service principal

## New Components Needed

### 1. GitHub Actions Workflow File

**Location:** `.github/workflows/release.yml`

**Key sections:**
```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'  # Trigger on version tags

jobs:
  build-and-release:
    runs-on: windows-latest

    steps:
      # Environment setup
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'npm'

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Setup MSVC
        uses: ilammy/msvc-dev-cmd@v1

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: src-tauri -> target

      # Build
      - name: Install dependencies
        run: npm install

      - name: Build and Release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'Scribe v__VERSION__'
          releaseBody: 'See CHANGELOG.md for details'
          releaseDraft: false
          prerelease: false

      # Post-build: Create MSIX
      - name: Install winapp CLI
        run: winget install Microsoft.WindowsAppSDK.CLI

      - name: Create MSIX package
        run: |
          cd src-tauri/target/release
          winapp init --name Scribe --publisher "Your Publisher"
          winapp manifest
          winapp package

      - name: Upload MSIX to release
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ steps.create_release.outputs.upload_url }}
          asset_path: ./Scribe.msix
          asset_name: Scribe_${{ github.ref_name }}_x64.msix
          asset_content_type: application/octet-stream
```

### 2. Code Signing Scripts

**Location:** `.github/scripts/sign.ps1`

**Purpose:** Centralize signing logic for both local and CI use

```powershell
param(
    [string]$FilePath,
    [string]$CertThumbprint,
    [string]$TimestampUrl = "http://timestamp.comodoca.com"
)

# Sign using Windows SDK signtool
signtool sign /fd sha256 /tr $TimestampUrl /td sha256 /sha1 $CertThumbprint $FilePath

if ($LASTEXITCODE -ne 0) {
    throw "Signing failed for $FilePath"
}

Write-Host "Successfully signed: $FilePath"
```

### 3. Website Repository

**Structure:**
```
scribe-website/
├── index.html          # Landing page
├── download.html       # Download page with installer links
├── privacy.html        # Privacy policy
├── assets/
│   ├── css/
│   ├── js/
│   └── images/
└── .github/
    └── workflows/
        └── deploy.yml  # Auto-deploy to hosting
```

**Hosting recommendation:** GitHub Pages (simplest) or Vercel (faster, better DX)

**Deploy workflow:**
```yaml
name: Deploy Website

on:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # For GitHub Pages
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./

      # OR for Vercel
      - name: Deploy to Vercel
        uses: amondnet/vercel-action@v20
        with:
          vercel-token: ${{ secrets.VERCEL_TOKEN }}
          vercel-org-id: ${{ secrets.VERCEL_ORG_ID }}
          vercel-project-id: ${{ secrets.VERCEL_PROJECT_ID }}
```

### 4. MSIX Configuration Files

**Location:** `src-tauri/msix/AppxManifest.xml` (generated by winapp, template for customization)

**Key elements:**
- Package identity (name, publisher, version)
- Capabilities (microphone access)
- Visual assets (tiles, icons)
- Entry point (scribe.exe)

### 5. Secrets Management

**Required GitHub Secrets:**

| Secret Name | Purpose | Format |
|-------------|---------|--------|
| `CERTIFICATE_PFX_BASE64` | Code signing certificate | Base64-encoded .pfx |
| `CERTIFICATE_PASSWORD` | Certificate password | Plain text |
| `AZURE_KEY_VAULT_URL` | Azure Key Vault URL | https://... |
| `AZURE_CLIENT_ID` | Azure service principal | GUID |
| `AZURE_CLIENT_SECRET` | Azure secret | String |
| `AZURE_TENANT_ID` | Azure tenant | GUID |
| `VERCEL_TOKEN` | Vercel deployment token | String |

**How to set:**
1. Go to GitHub repo → Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Add each secret with exact name and value

**Certificate encoding (for PFX approach):**
```bash
# On local machine with certificate
base64 -w 0 your-certificate.pfx > certificate-base64.txt
# Copy content of certificate-base64.txt to GitHub secret
```

## Data/Artifact Flow Diagram

```
Source Code (GitHub)
         │
         ▼
GitHub Actions Runner (windows-latest)
         │
         ├─→ whisper.cpp source (from crates.io via whisper-rs)
         │         │
         │         ▼
         │   CMake compile (MSVC + LLVM)
         │         │
         │         ▼
         │   whisper.dll (static lib)
         │         │
         ├─────────┘
         │
         ▼
Rust compilation (cargo build)
         │
         ▼
scribe.exe (unsigned)
         │
         ▼
Code Signing (Azure / PFX)
         │
         ▼
scribe.exe (signed)
         │
         ├─→ WiX Toolset → Scribe.msi
         │
         ├─→ NSIS → Scribe-setup.exe
         │
         └─→ winapp CLI → Scribe.msix (unsigned for Store)

         ▼
Distribution Channels:
         │
         ├─→ GitHub Releases (MSI + NSIS + latest.json)
         │         │
         │         └─→ Website download links
         │
         └─→ Microsoft Store Partner Center (MSIX upload)
                   │
                   └─→ Store signs MSIX with Microsoft cert
```

## Build Order & Implementation Strategy

### Phase 1: Basic CI/CD (Week 1)
**Goal:** Automated builds on GitHub Actions

**Tasks:**
1. Create `.github/workflows/build.yml` (test builds on PR)
2. Verify whisper.cpp compilation on windows-latest runner
3. Test tauri-action with minimal config
4. Confirm artifacts are generated

**Validation:** Successful build runs, downloadable .msi and .exe from workflow artifacts

### Phase 2: Code Signing (Week 2)
**Goal:** Signed installers to prevent SmartScreen warnings

**Decision point:** Traditional PFX vs Azure Code Signing
- **Start with PFX** if you already have a certificate (faster)
- **Migrate to Azure** for production (more secure, meets 2026 requirements)

**Tasks:**
1. Obtain code signing certificate (EV recommended for immediate reputation)
2. Configure signing in tauri.conf.json
3. Add certificate secrets to GitHub
4. Test signed build locally
5. Integrate signing into CI workflow

**Validation:** Signed .exe shows valid signature, no SmartScreen warning

### Phase 3: Release Automation (Week 2-3)
**Goal:** Tagged releases automatically create GitHub Releases

**Tasks:**
1. Create `.github/workflows/release.yml` (triggered by tags)
2. Configure tauri-action for release creation
3. Set up version tagging convention (v1.0.0 format)
4. Test full release cycle

**Validation:** Pushing `v1.0.0` tag creates release with MSI + NSIS + latest.json

### Phase 4: MSIX Packaging (Week 3)
**Goal:** Microsoft Store-ready MSIX package

**Tasks:**
1. Install winapp CLI in CI workflow
2. Create MSIX packaging script
3. Generate AppxManifest.xml template
4. Test MSIX installation locally
5. Upload MSIX to release artifacts

**Validation:** MSIX installs correctly, app appears in Start menu

### Phase 5: Store Submission (Week 4)
**Goal:** App listed on Microsoft Store

**Tasks:**
1. Create Microsoft Partner Center account
2. Reserve app name ("Scribe")
3. Configure Store listing (description, screenshots, privacy policy)
4. Submit MSIX package (unsigned)
5. Pass certification

**Manual process:** Store submission is NOT automated (API exists but complex)

**Validation:** App appears in Microsoft Store search results

### Phase 6: Website Deployment (Week 4)
**Goal:** Public-facing website with download links

**Tasks:**
1. Create simple static site (HTML + CSS + JS)
2. Set up GitHub Pages or Vercel
3. Configure auto-deployment on push
4. Add dynamic download links (point to latest GitHub Release)
5. Add privacy policy page (required for Store)

**Validation:** Website loads, download links work, privacy policy accessible

## Whisper.cpp Compilation on GitHub Actions

### Windows-Latest Runner Capabilities

**Pre-installed on windows-latest (2026):**
- Visual Studio 2022 with MSVC compiler
- CMake 3.x
- LLVM toolchain (includes llvm-rc needed for Windows resources)
- Git, PowerShell, Node.js

**What whisper-rs-sys needs:**
1. C++ compiler (MSVC) ✓ Available
2. CMake ✓ Available
3. LLVM/libclang (for Rust FFI bindings) ✓ Available

**No additional installation required** for basic whisper.cpp compilation.

### Build Configuration

**Environment variables to set:**
```yaml
env:
  LIBCLANG_PATH: C:\Program Files\LLVM\bin  # If not auto-detected
```

**Cargo build will automatically:**
1. Download whisper.cpp source (via whisper-rs-sys crate)
2. Invoke CMake to configure build
3. Compile whisper.cpp with MSVC
4. Link into Rust binary

**Potential issue:** Build time
- whisper.cpp compilation takes 5-10 minutes
- Solution: Use `swatinem/rust-cache@v2` to cache compiled artifacts
- Cache key includes Cargo.lock hash, so whisper.cpp only recompiles when dependencies change

### Verification Script

**Add to workflow for debugging:**
```yaml
- name: Verify build environment
  run: |
    cmake --version
    cl.exe 2>&1 | Select-String "Version"
    clang --version
    Write-Host "LIBCLANG_PATH: $env:LIBCLANG_PATH"
```

## Installer Generation Strategy

### Why Both MSI and NSIS?

**MSI advantages:**
- Enterprise-friendly (Group Policy deployment)
- Windows Installer service integration
- Mature, well-tested

**NSIS advantages:**
- Per-user install (no admin required)
- Smaller file size
- Better for consumer distribution
- ARM64 support (future-proofing)

**Recommendation:** Generate both, let users choose

### Configuration

**In tauri.conf.json:**
```json
{
  "bundle": {
    "targets": ["msi", "nsis"]
  }
}
```

**This generates:**
- `Scribe_1.0.0_x64.msi` (WiX)
- `Scribe_1.0.0_x64-setup.exe` (NSIS)

### Updater Compatibility

**Critical limitation:** Users who install via MSI can upgrade to NSIS, but NOT the reverse.

**Recommendation:**
- Default to NSIS for website downloads (easier for users)
- Provide MSI for enterprise users
- **Do NOT switch primary installer type after initial release**

**Updater JSON strategy:**
```json
{
  "version": "1.0.0",
  "platforms": {
    "windows-x86_64": {
      "signature": "...",
      "url": "https://github.com/.../Scribe_1.0.0_x64-setup.exe",
      "format": "nsis"
    }
  }
}
```

tauri-action generates this automatically if `uploadUpdaterJson: true` (default).

## Website Architecture

### Recommended Stack

**Platform:** Vercel (free tier)

**Why Vercel over GitHub Pages:**
- 30% faster deployment (2026 data)
- Better caching and CDN
- Automatic preview deployments for PRs
- Simple custom domain setup

**Alternative:** GitHub Pages if you want everything in one GitHub org (simpler secrets management)

### Site Structure

**Minimal viable site:**
```
/index.html          → Landing page (hero + features + download CTA)
/download.html       → Download options (MSI vs NSIS vs Store)
/privacy.html        → Privacy policy (required for Store listing)
/terms.html          → Terms of service (recommended)
/404.html            → Custom 404 page
```

**Assets:**
- CSS framework: TailwindCSS or simple custom CSS
- JavaScript: Vanilla JS to fetch latest release from GitHub API
- Images: Compressed PNGs/WebP

### Dynamic Download Links

**GitHub Releases API integration:**
```javascript
// Fetch latest release
fetch('https://api.github.com/repos/YOUR-USERNAME/scribe/releases/latest')
  .then(res => res.json())
  .then(data => {
    const msiAsset = data.assets.find(a => a.name.endsWith('.msi'));
    const nsisAsset = data.assets.find(a => a.name.endsWith('-setup.exe'));

    document.getElementById('download-msi').href = msiAsset.browser_download_url;
    document.getElementById('download-nsis').href = nsisAsset.browser_download_url;
  });
```

**Benefits:**
- No manual updates needed
- Always points to latest version
- Version number displayed automatically

### Deployment Automation

**For Vercel:**
```yaml
# .github/workflows/deploy-website.yml
name: Deploy Website

on:
  push:
    branches: [main]
    paths:
      - 'website/**'  # Only deploy when website files change

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: amondnet/vercel-action@v20
        with:
          vercel-token: ${{ secrets.VERCEL_TOKEN }}
          vercel-project-id: ${{ secrets.VERCEL_PROJECT_ID }}
          vercel-org-id: ${{ secrets.VERCEL_ORG_ID }}
          working-directory: ./website
```

**For GitHub Pages:**
```yaml
# .github/workflows/deploy-website.yml
name: Deploy Website

on:
  push:
    branches: [main]
    paths:
      - 'website/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./website
```

## Release Artifact Management

### Artifact Types

| Artifact | Use Case | Destination |
|----------|----------|-------------|
| `Scribe_x.x.x_x64.msi` | Enterprise users, traditional install | GitHub Releases |
| `Scribe_x.x.x_x64-setup.exe` | Consumer users, recommended | GitHub Releases + Website primary download |
| `Scribe_x.x.x_x64.msix` | Microsoft Store submission | Partner Center (manual upload) |
| `latest.json` | Auto-updater manifest | GitHub Releases |
| `*.sig` | Updater signature files | GitHub Releases |

### Naming Convention

**Tauri default:**
- MSI: `{productName}_{version}_{arch}.msi`
- NSIS: `{productName}_{version}_{arch}-setup.exe`

**Customize via tauri-action:**
```yaml
with:
  releaseAssetNamePattern: "Scribe_v__VERSION__-__TARGET__.__EXT__"
```

### Storage Strategy

**GitHub Releases (unlimited storage for public repos):**
- Primary distribution channel
- Automatic via tauri-action
- Releases API provides download URLs for website

**Microsoft Store (MSIX only):**
- Manual upload via Partner Center
- Store serves files to users
- No direct download link (Store app handles it)

**Website (no hosting of binaries):**
- Links to GitHub Releases
- Dynamic version fetching from API
- No need to update site for new releases

### Retention Policy

**GitHub Releases:**
- Keep all releases (no storage limit for public repos)
- Tag releases as "Latest" (automatic) and "Pre-release" (for beta)

**Microsoft Store:**
- Only latest version available
- Store manages rollback/versioning

## Secrets Management Strategy

### Certificate Secrets

**For PFX approach:**
```bash
# Generate base64-encoded certificate
base64 -w 0 certificate.pfx > cert.txt

# Add to GitHub Secrets:
# Name: CERTIFICATE_PFX_BASE64
# Value: (content of cert.txt)

# Name: CERTIFICATE_PASSWORD
# Value: (certificate password in plain text)
```

**Security note:** Base64 encoding is NOT encryption, it's just formatting. GitHub encrypts secrets at rest.

### Azure Code Signing Secrets

**Service Principal setup:**
1. Create Azure Key Vault
2. Upload certificate to Key Vault
3. Create Service Principal with Key Vault access
4. Add secrets to GitHub:
   - `AZURE_KEY_VAULT_URL`: `https://your-vault.vault.azure.net/`
   - `AZURE_CLIENT_ID`: Service principal application ID
   - `AZURE_CLIENT_SECRET`: Service principal secret
   - `AZURE_TENANT_ID`: Azure AD tenant ID

**Benefit:** Private key never leaves Azure, more secure than PFX file

### Website Deployment Secrets

**For Vercel:**
1. Create Vercel account
2. Install Vercel CLI: `npm i -g vercel`
3. Link project: `vercel link`
4. Get tokens: `vercel token create`
5. Get project/org IDs from `.vercel/project.json`
6. Add to GitHub Secrets:
   - `VERCEL_TOKEN`
   - `VERCEL_ORG_ID`
   - `VERCEL_PROJECT_ID`

**For GitHub Pages:**
- No additional secrets needed
- Uses built-in `GITHUB_TOKEN` (auto-provided)

### Secret Rotation

**Best practice:**
- Rotate certificate yearly (or when 460-day max expires)
- Rotate Azure service principal secrets every 90 days
- Rotate Vercel tokens yearly

**Process:**
1. Generate new secret/cert
2. Update GitHub Secret (overwrites old value)
3. Re-run workflow to test
4. Revoke old secret/cert after verification

## Microsoft Store Submission Process

### One-Time Setup

1. **Create Partner Center account**
   - Cost: $19 one-time fee for individual developers
   - Business verification required for organization accounts

2. **Reserve app name**
   - Name: "Scribe" (check availability first)
   - Identity name (e.g., "12345YourPublisher.Scribe")

3. **Configure app identity in AppxManifest.xml**
   - Must match reserved name EXACTLY
   - winapp CLI can generate this after account setup

### Submission Workflow

**Step 1: Prepare MSIX**
- Build signed .exe via GitHub Actions
- Use winapp CLI to package as MSIX
- **Do NOT sign MSIX** (Microsoft signs it)

**Step 2: Partner Center submission**
- Log in to Partner Center
- Create new submission for "Scribe"
- Fill out app listing:
  - Description
  - Screenshots (minimum 1, recommended 4+)
  - Privacy policy URL (required - host on website)
  - Age rating (ESRB/PEGI equivalent)
  - Category (Productivity)

**Step 3: Upload MSIX**
- Upload unsigned MSIX package
- Partner Center validates package
- Errors show in UI (fix and re-upload)

**Step 4: Certification**
- Submit for review
- Automated tests (1-2 hours)
- Manual review if flagged (1-3 days)
- App goes live after approval

### Automation Limitations

**What's NOT automated:**
- Initial submission (must be manual)
- App listing updates (description, screenshots)
- Certification workflow

**What CAN be automated (via API):**
- MSIX uploads for existing submissions
- Version updates
- Submission status checks

**Recommendation:** Start with manual submissions, automate later if shipping frequent updates.

### Store-Specific Requirements

**Privacy Policy:**
- Required URL field in submission
- Must be publicly accessible
- Should explain microphone permission usage
- Host on website (e.g., scribe-app.com/privacy)

**Age Rating:**
- Questionnaire in Partner Center
- Scribe likely rates "Everyone" (no violence, profanity, etc.)

**Capabilities Declaration:**
- MSIX manifest must declare microphone capability
- `<Capability Name="microphone" />`
- winapp CLI should auto-generate this

**WebView2:**
- Declare as dependency in MSIX
- Or bundle WebView2 bootstrapper (increases package size)

## Architecture Patterns to Follow

### Pattern 1: Fail Fast on Toolchain Issues

**Problem:** whisper.cpp compilation fails silently, wasting CI minutes

**Solution:** Validate environment before build
```yaml
- name: Validate build toolchain
  run: |
    cmake --version || exit 1
    cl.exe 2>&1 | Select-String "Version" || exit 1
    Write-Host "Toolchain validated"
```

### Pattern 2: Cache Aggressively

**Problem:** Every build recompiles whisper.cpp (5-10 minutes)

**Solution:** Multi-layer caching
```yaml
- name: Cache Rust dependencies
  uses: swatinem/rust-cache@v2
  with:
    workspaces: src-tauri -> target
    cache-on-failure: true

- name: Cache npm dependencies
  uses: actions/setup-node@v4
  with:
    cache: 'npm'
```

**Expected result:** First build 15 minutes, cached builds 3-5 minutes

### Pattern 3: Sign Then Package

**Problem:** Signing MSIX after packaging doesn't work for Store submission

**Correct order:**
1. Build .exe
2. Sign .exe
3. Package signed .exe into MSIX
4. Submit unsigned MSIX to Store

**Why:** Microsoft re-signs MSIX with their certificate. They need to see your .exe signature, not MSIX signature.

### Pattern 4: Version Single Source of Truth

**Problem:** Version in tauri.conf.json, Cargo.toml, AppxManifest.xml can drift

**Solution:** Version in tauri.conf.json is canonical
- Cargo.toml version is ignored by Tauri
- GitHub tag triggers release: `git tag v1.2.3`
- tauri-action reads version from tauri.conf.json
- winapp CLI can read version from manifest

**Automation:**
```yaml
# Extract version for use in workflow
- name: Get version
  id: version
  run: |
    $version = (Get-Content src-tauri/tauri.conf.json | ConvertFrom-Json).version
    echo "VERSION=$version" >> $env:GITHUB_OUTPUT
```

### Pattern 5: Separate Build and Release Workflows

**Build workflow (on every PR):**
- Runs tests
- Builds artifacts (unsigned)
- No release creation
- Fast feedback

**Release workflow (on version tag):**
- Full build
- Code signing
- MSIX packaging
- Create GitHub Release
- Upload artifacts

**Benefit:** PRs get fast build validation, releases get full pipeline

## Anti-Patterns to Avoid

### Anti-Pattern 1: Cross-Compilation for Windows

**What NOT to do:** Build Windows binaries on Linux/macOS runners

**Why it fails:**
- whisper.cpp requires native Windows toolchain
- WiX only runs on Windows
- NSIS cross-compilation is possible but fragile

**Correct approach:** Always use `runs-on: windows-latest` for Windows builds

### Anti-Pattern 2: Signing MSIX Before Store Submission

**What NOT to do:**
```yaml
# DON'T DO THIS
- name: Sign MSIX  # ❌ WRONG
  run: signtool sign package.msix
```

**Why it fails:** Microsoft Store re-signs packages with their certificate, invalidating your signature

**Correct approach:**
- Sign the .exe BEFORE packaging into MSIX
- Submit unsigned MSIX to Store

### Anti-Pattern 3: Hardcoded Download URLs in Website

**What NOT to do:**
```html
<!-- DON'T DO THIS -->
<a href="https://github.com/user/scribe/releases/download/v1.0.0/Scribe_1.0.0_x64.msi">
  Download MSI
</a>
```

**Why it breaks:** Every release requires manual website update

**Correct approach:** Use GitHub Releases API to fetch latest URL dynamically

### Anti-Pattern 4: Mixing MSI and NSIS for Updater

**What NOT to do:** Some users get MSI, some get NSIS, single updater endpoint

**Why it fails:** MSI users can't downgrade to NSIS later

**Correct approach:**
- Pick ONE primary installer (NSIS recommended)
- Updater JSON points to that format
- Provide MSI as alternative download only

### Anti-Pattern 5: Committing Secrets to Repo

**What NOT to do:**
```json
// DON'T DO THIS in tauri.conf.json
{
  "windows": {
    "certificateThumbprint": "ABC123..."  // ❌ NEVER commit
  }
}
```

**Why it's dangerous:** Exposes certificate details publicly

**Correct approach:** Use environment variables, set via GitHub Secrets

## Recommended Implementation Timeline

### Week 1: Foundation
- Day 1-2: Set up GitHub Actions build workflow
- Day 3-4: Verify whisper.cpp compilation in CI
- Day 5: Test artifact generation (MSI + NSIS)

### Week 2: Signing & Releases
- Day 1-2: Obtain code signing certificate
- Day 3: Configure signing (local testing)
- Day 4-5: Integrate signing into CI, create release workflow

### Week 3: MSIX & Website
- Day 1-2: winapp CLI integration for MSIX packaging
- Day 3-4: Build static website with dynamic download links
- Day 5: Deploy website to Vercel/GitHub Pages

### Week 4: Store Submission
- Day 1: Create Partner Center account, reserve app name
- Day 2-3: Prepare Store listing (description, screenshots, privacy policy)
- Day 4: Submit MSIX for certification
- Day 5: Address certification feedback if needed

### Week 5: Polish & Documentation
- Day 1-2: Write release documentation
- Day 3: Test full release cycle end-to-end
- Day 4-5: Create runbook for future releases

## Sources

### Code Signing & Certificates
- [Windows Code Signing | Tauri v2](https://v2.tauri.app/distribute/sign/windows/)
- [Code signing Windows apps with Azure Artifact service | DevClass](https://devclass.com/2026/01/14/code-signing-windows-apps-may-be-easier-and-more-secure-with-new-azure-artifact-service/)
- [Code Signing with Azure Key Vault | GlobalSign](https://support.globalsign.com/code-signing/code-signing-azure-key-vault-and-azure-signtool)
- [Azure SignTool GitHub Repository](https://github.com/vcsjones/AzureSignTool)
- [MSIX and CI/CD Pipeline signing with Azure Key Vault | Microsoft Learn](https://learn.microsoft.com/en-us/windows/msix/desktop/cicd-keyvault)

### MSIX Packaging
- [Microsoft winapp CLI Announcement | Windows Developer Blog](https://blogs.windows.com/windowsdeveloper/2026/01/22/announcing-winapp-the-windows-app-development-cli/)
- [winapp CLI GitHub Repository](https://github.com/microsoft/winappCli)
- [Tauri Issue #8548: Add MSIX generation](https://github.com/tauri-apps/tauri/issues/8548)
- [Building WSL-UI: The Microsoft Store Journey | Medium](https://medium.com/@ian.packard/building-wsl-ui-the-microsoft-store-journey-b808e61cb167)
- [Microsoft Store submission for MSIX | Microsoft Learn](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/create-app-submission)

### GitHub Actions & CI/CD
- [GitHub Actions for Tauri | Tauri v2](https://v2.tauri.app/distribute/pipelines/github/)
- [tauri-action GitHub Repository](https://github.com/tauri-apps/tauri-action)
- [MSVC Dev Command Action](https://github.com/ilammy/msvc-dev-cmd)
- [GitHub Actions Secrets Management | OneUpTime](https://oneuptime.com/blog/post/2026-01-25-github-actions-manage-secrets/view)
- [CMake VS 2026 on GitHub Actions | Scientific Computing](https://www.scivision.dev/github-actions-vs-2026)

### Installers & Distribution
- [Windows Installer | Tauri v2](https://v2.tauri.app/distribute/windows-installer/)
- [Tauri Discussion #8963: Release MSI and NSIS simultaneously](https://github.com/tauri-apps/tauri/discussions/8963)
- [Tauri Updater Plugin](https://v2.tauri.app/plugin/updater/)

### Website Hosting
- [GitHub Pages vs. Netlify | Netlify](https://www.netlify.com/github-pages-vs-netlify/)
- [Hosting Static Websites: GitHub Pages, Netlify, Vercel | NamasteDev](https://namastedev.com/blog/hosting-a-static-website-comparing-github-pages-netlify-and-vercel/)
- [Hugo Deployment Comparison 2026 | DasRoot](https://dasroot.net/posts/2026/01/hugo-deployment-netlify-vercel-cloudflare-pages-comparison/)

### whisper.cpp Compilation
- [whisper.cpp GitHub Repository](https://github.com/ggml-org/whisper.cpp)
- [whisper.cpp Windows Build Discussion](https://github.com/ggml-org/whisper.cpp/discussions/85)

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| GitHub Actions Build | HIGH | windows-latest has all required toolchain components |
| Code Signing | HIGH | Both PFX and Azure approaches well-documented |
| MSI/NSIS Generation | HIGH | Tauri's built-in bundling is mature and tested |
| MSIX Creation | MEDIUM | winapp CLI is new (Jan 2026), less battle-tested |
| Store Submission | MEDIUM | Process documented but manual steps involved |
| Website Deployment | HIGH | Static site hosting is straightforward |
| Secrets Management | HIGH | GitHub Secrets is standard practice |

## Open Questions & Validation Needed

1. **winapp CLI maturity:** Tool announced January 2026, may have rough edges. Test thoroughly before relying on it for production.

2. **MSIX testing:** Validate that MSIX created from Tauri .exe works correctly (all Win32 API calls, microphone access, etc.)

3. **Certificate cost:** Budget for code signing certificate ($100-400/year for OV, $300-600/year for EV)

4. **Store approval time:** First submission may take longer than subsequent updates. Plan for 1-2 week buffer.

5. **WebView2 in MSIX:** Verify WebView2 dependency handling in MSIX packages (may need to bundle or declare as Store dependency)

6. **ARM64 support:** If targeting ARM64 Windows devices, use NSIS (MSI doesn't support ARM64 in Tauri yet)

## Next Steps for Implementation

1. **Validate build environment:** Create minimal GitHub Actions workflow, confirm whisper.cpp compiles
2. **Acquire certificate:** Research code signing certificate providers, obtain certificate
3. **Test signing locally:** Sign .exe with signtool, verify signature validity
4. **Set up GitHub Secrets:** Add certificate and other secrets to repository
5. **Create release workflow:** Implement full pipeline from tauri.conf.json to GitHub Release
6. **Build website:** Create static site with dynamic download links
7. **Test MSIX creation:** Experiment with winapp CLI, validate MSIX functionality
8. **Partner Center setup:** Create account, reserve app name, prepare Store listing
9. **End-to-end test:** Full release cycle from git tag to Store submission
10. **Document process:** Create runbook for future releases
