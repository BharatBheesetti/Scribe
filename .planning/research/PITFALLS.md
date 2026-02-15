# Domain Pitfalls: Windows Packaging & Distribution

**Domain:** Desktop app packaging, distribution, and code signing for Windows
**Researched:** 2026-02-16
**Confidence:** MEDIUM (WebSearch-verified for general patterns, HIGH for Tauri-specific items)

**Context:** Adding packaging, distribution, and code signing to an existing Tauri v2 app with native C++ compilation (whisper.cpp), Win32 APIs, and runtime model downloads.

---

## Critical Pitfalls

Mistakes that cause rewrites, launch blockers, or major user-facing issues.

### Pitfall 1: Microsoft Store MSIX Packaging Not Yet Supported by Tauri

**What goes wrong:** Developers assume Tauri v2 can generate MSIX packages for Microsoft Store submission, then discover midway through planning that Tauri only generates MSI and EXE installers.

**Why it happens:** Microsoft Store documentation and ecosystem discussions often assume MSIX as the standard packaging format. Tauri's MS Store documentation focuses on "linking to unpacked application" workarounds rather than native MSIX support.

**Consequences:**
- Cannot use Microsoft Store's package extension features
- Must use workaround "unpacked application" linking (less integrated experience)
- Cannot leverage MSIX's AppContainer isolation (if that becomes a requirement)
- GitHub Issues [#4818](https://github.com/tauri-apps/tauri/issues/4818) and [#8548](https://github.com/tauri-apps/tauri/issues/8548) track MSIX feature requests but show no roadmap commitment

**Prevention:**
- Accept that Microsoft Store submission will use the "unpacked application" path (links to downloadable installer)
- Use NSIS or MSI for primary distribution channel
- If MSIX is critical requirement, evaluate non-Tauri alternatives or wait for feature support
- Monitor Tauri GitHub issues for MSIX roadmap updates

**Detection:**
- Check [Tauri v2 Microsoft Store guide](https://v2.tauri.app/distribute/microsoft-store/) for current limitations
- Search Tauri repo for "MSIX" to see latest discussion

**Phase:** Packaging (decide Store strategy vs direct-download-only early)

---

### Pitfall 2: Code Signing Reputation Cliff (Even with EV Certificates)

**What goes wrong:** Team purchases expensive EV code signing certificate expecting immediate SmartScreen trust, but users still see "Unknown publisher" warnings for weeks/months after launch.

**Why it happens:** **Microsoft changed SmartScreen behavior in March 2024.** EV certificates no longer grant instant reputation bypass. Both OV and EV certificates now require organic reputation building through download telemetry.

**Consequences:**
- Users abandon installation due to SmartScreen warnings (typical drop-off: 30-60% of non-technical users)
- Support burden from "Is this malware?" inquiries
- Negative brand perception ("looks sketchy")
- EV certificate premium cost ($300-600/year) provides minimal advantage over OV ($100-150/year)

**Prevention:**
1. **Expect 2-4 weeks minimum** for reputation building, even with EV cert
2. **Soft launch strategy:** Distribute to small trusted user group (50-100 users) first to build telemetry before public launch
3. **Submit binary to Microsoft manually** via [SmartScreen file submission form](https://www.microsoft.com/en-us/wdsi/filesubmission) to potentially expedite reputation
4. **Budget for reputation time** in launch timeline
5. **Consider OV certificate** unless EV required for other reasons (cost savings: ~$400/year)
6. **Never change publisher name or certificate** mid-stream (resets reputation to zero)

**Detection:**
- Test on fresh Windows VM without developer certificates installed
- Monitor support tickets for "Windows protected your PC" screenshots
- Track conversion funnel drop-off at installer download → first launch

**References:**
- [DigiCert: MS SmartScreen and Application Reputation](https://www.digicert.com/blog/ms-smartscreen-application-reputation)
- [Sectigo: MS SmartScreen and Application Reputation](https://support.sectigo.com/PS_KnowledgeDetailPageFaq?Id=kA01N000000zFJx)
- [Microsoft Q&A: Reputation with OV certificates](https://learn.microsoft.com/en-us/answers/questions/417016/reputation-with-ov-certificates-and-are-ev-certifi)

**Phase:** Code Signing + Launch Strategy

---

### Pitfall 3: Timestamp Server Failures Break Builds (No Retry Logic)

**What goes wrong:** CI/CD build randomly fails during code signing with "timestamp server could not be reached" error. Re-running the build succeeds. Happens 5-15% of builds.

**Why it happens:** Timestamp servers are external services (Sectigo, DigiCert, etc.) with occasional downtime or rate limits. Most signing tools (signtool.exe, Tauri bundler) don't implement automatic retry logic for timestamp failures.

**Consequences:**
- **Without timestamp:** Signed binaries expire when certificate expires (460 days max as of 2026)
- **Build brittleness:** CI/CD requires manual re-runs, blocks releases
- **User impact:** If you skip timestamping to avoid failures, users with old installers get "expired certificate" warnings after cert renewal

**Prevention:**
1. **Always timestamp** (critical for long-term binary validity)
2. **Implement retry logic in CI/CD:**
   ```bash
   # Example: retry signtool up to 3 times
   for i in {1..3}; do
     signtool sign /tr http://timestamp.sectigo.com /td sha256 ... && break
     sleep 10
   done
   ```
3. **Use multiple timestamp servers as fallback:**
   - Primary: `http://timestamp.sectigo.com` (Sectigo/Comodo)
   - Fallback: `http://timestamp.digicert.com` (DigiCert)
   - Fallback: `http://timestamp.globalsign.com` (GlobalSign)
4. **Monitor timestamp server health** in CI logs (track failure rates)
5. **Use RFC3161 protocol** (modern standard) over legacy Authenticode when possible

**Detection:**
- CI build logs show "The specified timestamp server either could not be reached or returned an invalid response"
- Signed binaries missing timestamp (check with `signtool verify /v /pa <file>`)

**References:**
- [DigiCert: Troubleshooting Timestamping Problems](https://knowledge.digicert.com/solution/SO912.html)
- [GitHub: electron-builder timestamp server failures](https://github.com/electron-userland/electron-builder/issues/2795)
- [List of free RFC3161 timestamp servers](https://gist.github.com/Manouchehri/fd754e402d98430243455713efada710)

**Phase:** CI/CD pipeline setup

---

### Pitfall 4: Certificate Validity Cliff (460-Day Max Since March 2026)

**What goes wrong:** Team plans for 3-year certificate lifecycle (legacy expectation), but new CA/Browser Forum rules limit code signing certificates to 460 days (~15 months) as of March 1, 2026. Certificate expires mid-product-cycle, requiring emergency renewal and re-signing.

**Why it happens:** Recent CA/Browser Forum baseline requirements changed maximum validity from 3 years to 460 days. Many developers unaware of this change.

**Consequences:**
- **Emergency renewals** disrupt development cycle every 15 months
- **Old installers break** if not timestamped correctly
- **Budget impact:** Annual renewal costs instead of 3-year amortization
- **Process burden:** Certificate installation, CI/CD credential updates, testing every 15 months

**Prevention:**
1. **Budget for annual renewals** (~$100-600/year depending on OV vs EV)
2. **Set calendar reminders 60 days before expiration** (procurement + installation lead time)
3. **Always timestamp signed binaries** (allows them to remain valid post-expiration)
4. **Automate certificate rotation** in CI/CD:
   - Use Azure Key Vault or GitHub Secrets for certificate storage
   - Document certificate update procedure
   - Test certificate replacement in staging before production
5. **Consider cloud-based signing** (Azure Code Signing) to reduce manual cert management

**Detection:**
- Check certificate expiration: `signtool verify /v /pa <signed-binary>`
- Monitor certificate expiration dates in CI/CD secrets

**References:**
- [SSL2Buy: Code Signing Certificate Validity Reduced to 460 Days](https://www.ssl2buy.com/wiki/code-signing-certificate-validity-reduced-to-460-days)
- [DigiCert Code Signing FAQs 2026 Guide](https://comparecheapssl.com/digicert-code-signing-frequently-asked-questions-faqs/)

**Phase:** Code Signing + Operations

---

### Pitfall 5: Microsoft Store Runtime Download Policy Violations

**What goes wrong:** App downloads 40MB-1.5GB ML models at runtime. Microsoft Store rejects submission for "incomplete package" or requires special capabilities/permissions that break functionality.

**Why it happens:** Microsoft Store policies historically discourage large post-install downloads without clear user consent. Models downloaded to `%APPDATA%` may trigger sandbox restrictions or policy flags.

**Consequences:**
- Store submission rejection (policy violation)
- If approved, Store may require intrusive permission prompts that degrade UX
- AppContainer sandbox may block writes to `%APPDATA%` without explicit capability declarations

**Prevention:**
1. **Validate Store compatibility early:**
   - Check [Microsoft Store Policies 7.19](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies) section on package completeness
   - Submit test build early in development to identify policy issues
2. **Leverage Windows ML Model Catalog APIs** (if applicable):
   - [Windows ML Model Catalog](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/model-catalog/overview) allows dynamic model downloads with OS-level caching
   - Microsoft-sanctioned approach for ML model distribution
3. **Clear user consent for large downloads:**
   - Show download size, purpose, and progress before initiating
   - Allow user to cancel or defer download
   - Store policy requires transparency for downloads >100MB
4. **Test with MSIX capabilities:**
   - If using fullTrust MSIX (when Tauri supports it), verify `%APPDATA%` write access
   - May need `broadFileSystemAccess` capability
5. **Alternative: Smaller base models in package, large models optional**
   - Ship tiny-en (40MB) in installer
   - Download larger models (base, small, medium) on-demand

**Detection:**
- Test Store submission with early alpha build
- Monitor Store review feedback for policy violations
- Test model download on fresh Windows install without admin rights

**References:**
- [Windows ML Model Catalog Overview](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/model-catalog/overview)
- [Microsoft Store Policies 7.19](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies)

**Phase:** Microsoft Store submission preparation

---

## Moderate Pitfalls

Mistakes that cause delays, technical debt, or operational friction.

### Pitfall 6: whisper.cpp CI/CD Build Complexity (MSVC + LLVM + CMake)

**What goes wrong:** GitHub Actions build fails with "LLVM not found," "CMake version too old," or "MSVC linker error" when compiling whisper.cpp from source via whisper-rs.

**Why it happens:** whisper.cpp requires specific Windows toolchain:
- LLVM/libclang (for whisper-rs-sys bindings)
- CMake 3.15+ (for whisper.cpp build)
- MSVC 19.x (Visual Studio 2019/2022 C++ toolchain)
- Rust toolchain (for Tauri)

GitHub Actions `windows-latest` runners include some but not all dependencies. Versions drift over time.

**Consequences:**
- Build failures block CI/CD
- Inconsistent builds between local dev and CI
- Long troubleshooting cycles (dependency matrix is complex)

**Prevention:**
1. **Pin GitHub Actions runner version:**
   ```yaml
   runs-on: windows-2022  # Specific version, not windows-latest
   ```
2. **Explicitly install required toolchain:**
   ```yaml
   - name: Install LLVM
     run: choco install llvm --version=15.0.7
   - name: Set LIBCLANG_PATH
     run: echo "LIBCLANG_PATH=C:\Program Files\LLVM\bin" >> $GITHUB_ENV
   - name: Setup MSVC
     uses: microsoft/setup-msbuild@v1.1
   ```
3. **Cache Rust build artifacts:**
   ```yaml
   - uses: Swatinem/rust-cache@v2
     with:
       key: ${{ runner.os }}-rust-${{ hashFiles('**/Cargo.lock') }}
   ```
4. **Cache whisper.cpp build:**
   - Use `actions/cache@v3` for `target/` directory
   - Reduces build time from ~15min to ~3min on cache hit
5. **Test CI config locally:**
   - Use `act` tool to run GitHub Actions locally before pushing
6. **Document exact dependency versions** in README for reproducibility

**Detection:**
- Build fails with "could not find libclang" or CMake errors
- Inconsistent build results between runs
- Build time >10 minutes (indicates cache miss)

**References:**
- [actions-rust-lang/setup-rust-toolchain](https://github.com/actions-rust-lang/setup-rust-toolchain)
- [Swatinem/rust-cache](https://github.com/marketplace/actions/rust-cache)
- [whisper.cpp Windows build discussion](https://github.com/ggml-org/whisper.cpp/discussions/85)

**Phase:** CI/CD pipeline setup

---

### Pitfall 7: WebView2 Runtime Distribution Choice Impacts Installer Size and Compatibility

**What goes wrong:** Developer chooses wrong WebView2 distribution mode, resulting in either bloated installer (~180MB) or installation failures on fresh Windows installs.

**Why it happens:** Tauri offers 4 WebView2 distribution modes with unclear tradeoffs:
1. **Download Bootstrapper** (default): Requires internet, smallest installer
2. **Embedded Bootstrapper**: ~1.8MB overhead, better Windows 7 support
3. **Offline Installer**: ~127MB overhead, works offline
4. **Fixed Version**: ~180MB overhead, specific WebView2 version bundled

Most developers stick with default without evaluating tradeoffs.

**Consequences:**
- **Download Bootstrapper:** Installation fails on offline/air-gapped systems
- **Offline/Fixed Version:** Installer bloat (127-180MB) for 5MB app discourages downloads
- **Fixed Version:** Security/update burden (must re-release app for WebView2 updates)

**Prevention:**
1. **For Scribe (privacy-focused, ~5MB app):** Use **Download Bootstrapper** (default)
   - Rationale: Users downloading desktop app already have internet
   - WebView2 auto-updates via Windows Update (security benefit)
   - Keeps installer size <10MB
2. **For enterprise/offline deployments:** Use **Offline Installer**
   - Rationale: Enterprise networks may block external downloads
   - One-time 127MB cost acceptable for corporate deployments
3. **Never use Fixed Version** unless specific WebView2 bug requires pinned version
   - Security nightmare (must re-release for every WebView2 CVE)
   - 180MB installer size
4. **Test on fresh Windows 10/11 VM** without WebView2 pre-installed
5. **Document WebView2 requirement** in system requirements

**Detection:**
- Installer size unexpectedly large (>50MB for small app)
- Installation failures on fresh Windows VMs
- Users report "WebView2 not found" errors

**References:**
- [Tauri Windows Installer WebView2 Options](https://v2.tauri.app/distribute/windows-installer/)
- [Microsoft: Distribute WebView2 Runtime](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)
- [WebView2 Evergreen vs Fixed Version](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/evergreen-vs-fixed-version)

**Phase:** Installer configuration

---

### Pitfall 8: NSIS vs WiX Installer Choice Impacts Features and Maintainability

**What goes wrong:** Team chooses installer format (NSIS vs WiX) based on unfamiliarity or cargo-cult recommendations, later discovers it lacks needed features (e.g., COM registration, custom upgrade logic).

**Why it happens:** Tauri defaults to NSIS for `.exe` installers and WiX for `.msi`. Developers rarely evaluate which format fits their needs.

**Consequences:**
- **NSIS limitations:** No built-in upgrade detection, manual registry manipulation for COM/extensions
- **WiX limitations:** Steeper learning curve, XML verbosity, limited runtime download flexibility
- **Refactoring cost:** Switching mid-project requires rewriting custom actions/hooks

**Prevention:**
1. **Choose NSIS if:**
   - Need custom UI/branding
   - Runtime dependency downloads (WebView2, VC++ Redistributables)
   - Simpler scripting model (imperative vs declarative)
   - Fast iteration (compile time <5s vs WiX's ~30s)
2. **Choose WiX if:**
   - Enterprise deployment (Group Policy requires MSI)
   - Need Windows Installer features (rollback, patching, COM registration)
   - Upgrade/downgrade logic complexity
3. **For Scribe:** **NSIS** recommended
   - Rationale: Custom hotkey capture UI, runtime model downloads, no COM dependencies
   - NSIS hooks support pre/post-install model download scripts
4. **Test upgrade path early:**
   - Install v1.0.0, then upgrade to v1.1.0
   - Verify settings preserved, old files removed, shortcuts updated
5. **Document installer customizations** for future maintainers

**Detection:**
- Feature gap discovered mid-development (e.g., "How do I detect previous install in NSIS?")
- Installer build time >30s (WiX compile overhead)

**References:**
- [Tauri Windows Installer: NSIS vs WiX](https://v2.tauri.app/distribute/windows-installer/)
- [WiX Toolset Documentation](https://wixtoolset.org/docs/)
- [NSIS Documentation](https://nsis.sourceforge.io/Docs/)

**Phase:** Installer architecture decision

---

### Pitfall 9: GitHub Actions Artifact Storage Quota Exhaustion

**What goes wrong:** CI/CD uploads every build's signed binaries (50-200MB each) as artifacts. After 2-3 months, GitHub storage quota exceeded, blocking new builds.

**Why it happens:** GitHub Actions includes limited artifact storage per plan:
- Free tier: 500MB storage, 2GB bandwidth/month
- Pro: 2GB storage, 10GB bandwidth/month
- Team: 5GB storage, 50GB bandwidth/month

Artifacts default to 90-day retention. Large Windows binaries (with bundled WebView2 or models) accumulate quickly.

**Consequences:**
- Builds fail with "Artifact storage quota has been hit"
- Must manually delete old artifacts to unblock pipeline
- Storage quota usage updates lag 6-12 hours (blind troubleshooting)

**Prevention:**
1. **Reduce artifact retention:**
   ```yaml
   - uses: actions/upload-artifact@v3
     with:
       name: scribe-installer
       path: target/release/bundle/nsis/*.exe
       retention-days: 7  # Down from default 90
   ```
2. **Only upload release builds, not every commit:**
   ```yaml
   if: startsWith(github.ref, 'refs/tags/v')  # Only on version tags
   ```
3. **Upload to external storage for long-term archives:**
   - Use S3, Azure Blob, or GitHub Releases for tagged versions
   - Keep GitHub Artifacts for short-term CI artifacts only
4. **Monitor storage usage:**
   - Check Settings → Billing → Storage usage monthly
   - Set up alerts before quota exhaustion
5. **Don't bundle large models in installer** (download at runtime instead)
   - Keeps installer <10MB
   - Reduces artifact storage 10-20x

**Detection:**
- CI fails with "Artifact storage quota has been hit"
- Billing dashboard shows storage near quota limit
- Artifact uploads succeed but downloads fail (bandwidth quota)

**References:**
- [GitHub Actions Artifact Storage Limits](https://medium.com/@aayushpaigwar/understanding-github-actions-artifact-storage-limits-and-how-to-manage-them-a577939f1c57)
- [Avoiding GitHub Actions Storage Quota](https://thomasbillington.co.uk/2023/03/05/github-actions-storage-limits.html)

**Phase:** CI/CD optimization

---

### Pitfall 10: Installer Testing on Dirty Systems (Not Fresh VMs)

**What goes wrong:** Installer works perfectly on developer machines and CI but fails on 10% of user systems with cryptic errors ("DLL not found," "Access denied," "Installation failed").

**Why it happens:** Developer machines have accumulated dependencies (VC++ Redistributables, .NET runtimes, admin rights, disabled UAC). Installer implicitly relies on these. Fresh user systems lack them.

**Consequences:**
- High support burden from installation failures
- Negative reviews ("doesn't install")
- Users abandon product before first run
- Debugging requires reproducing on fresh systems (time-consuming)

**Prevention:**
1. **Test on fresh Windows VMs before every release:**
   - Windows 10 Home (most restrictive UAC)
   - Windows 11 Pro (latest OS)
   - Windows 10 LTSC (enterprise, no Store)
2. **Use VM snapshots for repeatability:**
   - Create "Clean Windows 10" snapshot
   - Install app, test, revert to snapshot
   - Repeat for each build
3. **Test as non-admin user:**
   - Create standard user account on VM
   - Verify installer works without admin elevation (if designed for per-user install)
4. **Checklist for installer testing:**
   - [ ] Fresh Windows 10 install (no updates)
   - [ ] Fresh Windows 11 install (latest updates)
   - [ ] Non-admin user account
   - [ ] Offline (no internet) if using Offline WebView2 installer
   - [ ] Upgrade from previous version (v1.0 → v1.1)
   - [ ] Uninstall (verify clean removal)
   - [ ] Reinstall (verify no leftover artifacts)
   - [ ] Custom install path (C:\CustomPath instead of default)
5. **Monitor user-reported installation errors** via telemetry or support tickets

**Detection:**
- User reports "Installation failed" without clear error message
- Works on dev machine, fails on user machines
- High support ticket volume post-release

**References:**
- [Software Installation Testing Guide](https://www.softwaretestinghelp.com/software-installationuninstallation-testing/)
- [Advanced Installer: Installer Testing Guide](https://www.advancedinstaller.com/application-packaging-testing-process-guide.html)

**Phase:** QA/Release process

---

### Pitfall 11: Dual Distribution Version Drift (Store vs Direct Download)

**What goes wrong:** Team maintains two distribution channels (Microsoft Store + direct download website). Versions diverge due to Store review delays (3-7 days), users report inconsistent behavior, support burden doubles.

**Why it happens:** Microsoft Store submissions require review (policy compliance, security scan). Direct downloads can be released instantly. Teams don't plan for version skew.

**Consequences:**
- **User confusion:** "I have v1.2.0 but forum says v1.3.0 is out"
- **Support complexity:** "Which version do you have? Where did you download it?"
- **Feature parity issues:** Store version missing bugfix for days
- **Analytics fragmentation:** Two codebases, two telemetry streams

**Prevention:**
1. **Version numbering strategy:**
   - Use 4-part version: `Major.Minor.Patch.Build`
   - Store builds: `1.2.3.0` (last digit always 0)
   - Direct downloads: `1.2.3.1`, `1.2.3.2`, etc. (hotfixes)
   - Makes distribution channel visible in version number
2. **Synchronized release process:**
   - Submit to Store 5-7 days before planned release date
   - Hold direct download release until Store approval
   - Release both simultaneously
3. **Hotfix strategy:**
   - Critical bugs: Release direct download immediately, submit Store update
   - Non-critical: Batch into next synchronized release
4. **Update notifications:**
   - In-app update checker points to correct channel
   - Store version checks Store API, direct version checks website
5. **Analytics tagging:**
   - Tag telemetry with distribution channel (`store` vs `direct`)
   - Track version adoption per channel
6. **Document version strategy** in README and support docs

**Detection:**
- Users report "Your website says v1.3 but Store only has v1.2"
- Support tickets mention version-specific bugs already fixed in other channel
- Analytics show two distinct version cohorts

**References:**
- [Microsoft Store App Package Versioning](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/package-version-numbering)

**Phase:** Release management + Operations

---

## Minor Pitfalls

Mistakes that cause annoyance, confusion, or small delays but are easily fixable.

### Pitfall 12: Missing Privacy Policy Blocks Store Submission

**What goes wrong:** Store submission rejected for "Missing privacy policy" despite app collecting no user data.

**Why it happens:** Microsoft Store requires privacy policy if app:
- Accesses internet (even for model downloads)
- Writes to `%APPDATA%` (stores user preferences)
- Uses microphone (Scribe's core feature)

"We don't collect data" is not an exemption from policy requirement.

**Consequences:**
- Store submission rejected immediately
- 3-7 day delay to create policy + resubmit
- Blocks launch timeline

**Prevention:**
1. **Create privacy policy before first Store submission**
2. **Host at stable URL** (e.g., `https://scribe-app.com/privacy`)
3. **Template for local-only app:**
   ```
   Privacy Policy for Scribe

   Scribe is a local-first voice-to-text application. We collect no user data.

   Data Storage:
   - All voice recordings and transcriptions are processed locally on your device
   - Transcription history is stored in %APPDATA%/Scribe/history.json
   - Model files are stored in %APPDATA%/Scribe/models/
   - No data is transmitted to external servers

   Permissions:
   - Microphone: Required for voice input
   - File System: Stores settings and history locally

   Contact: support@scribe-app.com
   ```
4. **Link in Tauri config:**
   ```json
   "bundle": {
     "publisher": "Your Name",
     "privacyPolicyUrl": "https://scribe-app.com/privacy"
   }
   ```
5. **Keep policy separate from Terms of Use** (Store rejects combined docs)

**Detection:**
- Store submission review feedback: "Privacy policy missing or invalid"

**References:**
- [Microsoft Store Policies: Privacy Policy Requirements](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies)
- [Microsoft Q&A: About Providing a Privacy Policy](https://learn.microsoft.com/en-us/answers/questions/8937/about-providing-a-privacy-policy)

**Phase:** Store submission preparation

---

### Pitfall 13: NSIS Installer UAC Elevation Confusion (User vs Machine Install)

**What goes wrong:** NSIS installer requests admin rights, but app only needs user-level permissions. Users decline UAC prompt, installation fails.

**Why it happens:** NSIS defaults to `RequestExecutionLevel admin` if not explicitly configured. Tauri may inherit this default.

**Consequences:**
- Corporate users cannot install (no admin rights)
- UAC prompt scares non-technical users
- Unnecessary elevation violates least-privilege principle

**Prevention:**
1. **For per-user apps (like Scribe):** Configure NSIS for `currentUser` install:
   ```json
   "tauri": {
     "bundle": {
       "windows": {
         "nsis": {
           "installMode": "currentUser"  // No admin required
         }
       }
     }
   }
   ```
2. **Install to `%LOCALAPPDATA%` instead of `Program Files`:**
   - `C:\Users\<username>\AppData\Local\Scribe\` (no admin needed)
3. **Test on standard user account** (not admin) to verify
4. **Document installation requirements** in README

**Detection:**
- Users report "Installation requires admin rights" when it shouldn't
- Corporate users cannot install
- Installer shows UAC prompt unexpectedly

**References:**
- [NSIS Reference: RequestExecutionLevel](https://nsis.sourceforge.io/Reference/RequestExecutionLevel)
- [Tauri Windows Installer: Install Modes](https://v2.tauri.app/distribute/windows-installer/)

**Phase:** Installer configuration

---

### Pitfall 14: Certificate Export Password Lost (Cannot Sign New Builds)

**What goes wrong:** Team member who set up code signing leaves. New builds fail because PFX certificate password is unknown or stored only in departing developer's password manager.

**Why it happens:** Code signing setup is one-time task, password documentation overlooked.

**Consequences:**
- Cannot sign new releases until new certificate purchased
- Certificate revocation process required (if password truly lost)
- Release delays while re-procuring certificate

**Prevention:**
1. **Store certificate password in team password manager:**
   - 1Password, Bitwarden, or equivalent
   - Share with at least 2 team members
2. **Document certificate details:**
   - Certificate authority (DigiCert, Sectigo, etc.)
   - Purchase date and expiration
   - Associated email/account
3. **Use CI/CD secrets for automation:**
   - GitHub Actions: Encrypted secrets
   - Azure DevOps: Variable groups
4. **Test certificate restoration from backup:**
   - Export PFX, delete from machine, re-import with password
   - Verify signing works
5. **Set calendar reminder for renewal** 60 days before expiration

**Detection:**
- Build fails with "Cannot access certificate" or "Invalid password"
- Only one person can create release builds

**Phase:** Code signing setup + Operations

---

### Pitfall 15: Auto-Update Mechanism Not Planned from v1.0

**What goes wrong:** App launches successfully, gains users, then team realizes updating requires users to manually download new installer. Adoption of new versions <20%.

**Why it happens:** Auto-update is "nice to have" feature, deferred to post-MVP. But retrofitting is complex and requires breaking changes to v1.0 architecture.

**Consequences:**
- Users stuck on old versions with bugs
- Cannot push critical security updates
- Must support multiple version cohorts indefinitely
- Retrofitting requires v2.0 (breaking change)

**Prevention:**
1. **Plan auto-update from v1.0**, even if not implemented:
   - Reserve JSON endpoint for version manifest (e.g., `https://scribe-app.com/version.json`)
   - Include version check logic in v1.0 (even if it just logs "Update available")
2. **Use Tauri updater plugin:**
   - [tauri-plugin-updater](https://v2.tauri.app/plugin/updater/) for built-in support
   - Supports silent background downloads, user-prompted updates
3. **Version manifest example:**
   ```json
   {
     "version": "1.2.0",
     "notes": "Bug fixes and performance improvements",
     "pub_date": "2026-02-16T12:00:00Z",
     "platforms": {
       "windows-x86_64": {
         "signature": "...",
         "url": "https://scribe-app.com/downloads/Scribe-1.2.0-setup.exe"
       }
     }
   }
   ```
4. **NSIS silent update support:**
   - Configure installer to support `/S` silent flag
   - Auto-updater can invoke `Scribe-1.2.0-setup.exe /S` without user interaction
5. **Test update flow:**
   - Install v1.0, run auto-updater to v1.1
   - Verify settings/data preserved

**Detection:**
- Analytics show users stuck on old versions (6+ months old)
- Support tickets for bugs already fixed in new version
- No mechanism to push security patches

**References:**
- [Tauri Updater Plugin](https://v2.tauri.app/plugin/updater/)
- [NSIS Silent Install](https://nsis.sourceforge.io/Reference/SilentInstall)

**Phase:** Architecture planning (v1.0)

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation | Severity |
|-------------|---------------|------------|----------|
| **Code Signing Setup** | Timestamp server failures break builds | Implement retry logic with multiple timestamp URLs | HIGH |
| **Code Signing Setup** | Certificate password lost | Store in team password manager, document | MEDIUM |
| **Code Signing Setup** | SmartScreen reputation takes weeks even with EV cert | Soft launch with trusted users first, submit to Microsoft manually | CRITICAL |
| **Installer Config** | Wrong WebView2 distribution mode (bloated installer or offline failures) | Choose Download Bootstrapper for online app, test on fresh VM | MEDIUM |
| **Installer Config** | NSIS vs WiX choice blocks features later | Evaluate before implementation; NSIS for Scribe (custom UI, runtime downloads) | MEDIUM |
| **Installer Config** | UAC elevation when not needed | Set `installMode: currentUser` for per-user apps | LOW |
| **CI/CD Pipeline** | whisper.cpp build fails (missing LLVM/CMake/MSVC) | Pin runner version, explicitly install toolchain, cache builds | HIGH |
| **CI/CD Pipeline** | Artifact storage quota exhaustion | Reduce retention to 7 days, only upload tagged releases, use external storage | MEDIUM |
| **Microsoft Store** | Tauri doesn't support MSIX packaging | Use "unpacked application" workaround or defer Store submission | CRITICAL |
| **Microsoft Store** | Runtime model downloads violate Store policy | Test early submission, use Windows ML Model Catalog if possible, transparent UI | HIGH |
| **Microsoft Store** | Missing privacy policy blocks submission | Create policy before first submission, host at stable URL | LOW |
| **Website/Landing Page** | No download analytics (cannot measure conversion) | Add simple analytics (Plausible, self-hosted), track download → install → first-run | MEDIUM |
| **Website/Landing Page** | Feature-focused messaging instead of problem/solution | Lead with user pain point ("Tired of typing?"), then solution | MEDIUM |
| **Release Management** | Store vs direct download version drift | Synchronized release process, version numbering strategy (4-part version) | MEDIUM |
| **Release Management** | No auto-update mechanism (users stuck on old versions) | Plan updater from v1.0, use tauri-plugin-updater | HIGH |
| **QA/Testing** | Installer only tested on developer machines | Test on fresh Windows VMs (Win10 Home + Win11 Pro), non-admin user | HIGH |
| **QA/Testing** | Upgrade path not tested (v1.0 → v1.1) | Test upgrade before each release, verify settings preserved | MEDIUM |

---

## Sources

### MSIX and Microsoft Store
- [Microsoft Store | Tauri](https://v2.tauri.app/distribute/microsoft-store/)
- [Tauri GitHub Issue #4818: MSIX Packages](https://github.com/tauri-apps/tauri/issues/4818)
- [Tauri GitHub Issue #8548: Generate MSIX/APPX](https://github.com/tauri-apps/tauri/issues/8548)
- [Microsoft: MSIX AppContainer Apps](https://learn.microsoft.com/en-us/windows/msix/msix-container)
- [Microsoft: App Capability Declarations](https://learn.microsoft.com/en-us/windows/uwp/packaging/app-capability-declarations)
- [Microsoft Store Policies 7.19](https://learn.microsoft.com/en-us/windows/apps/publish/store-policies)
- [Advanced Installer: MSIX Technology Fundamentals](https://www.advancedinstaller.com/application-packaging-training/msix-packaging/ebook/modern-technology.html)

### Code Signing and SmartScreen
- [Tauri: Windows Code Signing](https://v2.tauri.app/distribute/sign/windows/)
- [DigiCert: MS SmartScreen and Application Reputation](https://www.digicert.com/blog/ms-smartscreen-application-reputation)
- [Sectigo: MS SmartScreen and Application Reputation](https://support.sectigo.com/PS_KnowledgeDetailPageFaq?Id=kA01N000000zFJx)
- [Microsoft Q&A: Reputation with OV certificates](https://learn.microsoft.com/en-us/answers/questions/417016/reputation-with-ov-certificates-and-are-ev-certifi)
- [SSL2Buy: Code Signing Certificate Validity Reduced to 460 Days](https://www.ssl2buy.com/wiki/code-signing-certificate-validity-reduced-to-460-days)
- [DigiCert: Troubleshooting Timestamping Problems](https://knowledge.digicert.com/solution/SO912.html)
- [Microsoft: Time Stamping Authenticode Signatures](https://learn.microsoft.com/en-us/windows/win32/seccrypto/time-stamping-authenticode-signatures)

### Windows Installer (NSIS/WiX)
- [Tauri: Windows Installer](https://v2.tauri.app/distribute/windows-installer/)
- [NSIS: Best Practices](https://nsis.sourceforge.io/Best_practices)
- [Microsoft: Windows Installer Best Practices](https://learn.microsoft.com/en-us/windows/win32/msi/windows-installer-best-practices)
- [NSIS: RequestExecutionLevel](https://nsis.sourceforge.io/Reference/RequestExecutionLevel)
- [Advanced Installer: Installer Testing Guide](https://www.advancedinstaller.com/application-packaging-testing-process-guide.html)

### WebView2 Distribution
- [Microsoft: Distribute WebView2 Runtime](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)
- [Microsoft: Evergreen vs Fixed Version](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/evergreen-vs-fixed-version)

### CI/CD and Build Infrastructure
- [GitHub: actions-rust-lang/setup-rust-toolchain](https://github.com/actions-rust-lang/setup-rust-toolchain)
- [GitHub: Swatinem/rust-cache](https://github.com/marketplace/actions/rust-cache)
- [whisper.cpp: Windows Build Discussion](https://github.com/ggml-org/whisper.cpp/discussions/85)
- [GitHub Actions Artifact Storage Limits](https://medium.com/@aayushpaigwar/understanding-github-actions-artifact-storage-limits-and-how-to-manage-them-a577939f1c57)
- [Avoiding GitHub Actions Storage Quota](https://thomasbillington.co.uk/2023/03/05/github-actions-storage-limits.html)

### Windows ML and Runtime Downloads
- [Windows ML Model Catalog Overview](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/model-catalog/overview)
- [Microsoft: What is Windows ML?](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/overview)

### Landing Page Best Practices
- [High-Converting SaaS Landing Pages 2026](https://www.saashero.net/design/enterprise-landing-page-design-2026/)
- [20 Best SaaS Landing Pages + Best Practices](https://fibr.ai/landing-page/saas-landing-pages)
- [Skyrocket SaaS Website Conversions 2026](https://www.webstacks.com/blog/website-conversions-for-saas-businesses)

---

## Confidence Assessment

| Area | Confidence | Reason |
|------|------------|--------|
| **MSIX Limitations** | HIGH | Verified via Tauri docs + GitHub issues (no MSIX support as of Feb 2026) |
| **SmartScreen Reputation** | HIGH | WebSearch findings verified across multiple sources (DigiCert, Sectigo, Microsoft Q&A) |
| **Code Signing Pitfalls** | HIGH | Tauri official docs + CA vendor documentation |
| **whisper.cpp CI/CD** | MEDIUM | WebSearch findings + general Rust/C++ build knowledge; needs validation with actual whisper-rs build |
| **Store Policies** | MEDIUM | WebSearch findings from Microsoft docs; specific Scribe scenario needs early submission test |
| **Installer Testing** | HIGH | Industry-standard practices verified across multiple sources |
| **Landing Page Pitfalls** | LOW | Generic SaaS advice; desktop app download pages have different patterns |

---

## Gaps and Open Questions

**Items requiring phase-specific research:**

1. **Microsoft Store submission dry run:**
   - Submit alpha build to Store early to surface actual policy issues
   - Test runtime model download behavior in Store-installed app
   - Validate "unpacked application" distribution path works for Scribe

2. **Code signing reputation timeline:**
   - Track actual SmartScreen reputation building for Scribe specifically
   - Measure user drop-off due to SmartScreen warnings
   - Test manual Microsoft submission process

3. **whisper.cpp build matrix:**
   - Validate exact LLVM/CMake/MSVC versions required for whisper-rs on GitHub Actions
   - Test build cache effectiveness (time savings)
   - Document reproducible build environment

4. **Landing page conversion:**
   - Research desktop app download page best practices (different from SaaS)
   - A/B test messaging: problem-first vs feature-first
   - Track download → install → first-run funnel

5. **Dual distribution version management:**
   - Finalize version numbering strategy (4-part version, channel tagging)
   - Document release process for synchronized Store + direct distribution
   - Plan hotfix strategy for critical bugs

**NOT pitfalls but considerations for later phases:**
- Telemetry/analytics strategy (privacy-preserving, local-first)
- Crash reporting mechanism
- Support channel setup (email, Discord, GitHub Discussions)
- Documentation site (separate from landing page)
