# Application update strategy

Source Weaver does not enable automatic self-updates yet. The safe current path is manual update discovery, artifact verification, and user-initiated install or replacement.

Automatic updates should be implemented only after signed release artifacts are enforced for the update channel. `docs/code-signing.md` defines the signing hooks and required secrets, but the current repository state has no real Windows code-signing certificate or OpenPGP release key configured.

## Research notes

Tauri's updater documentation checked on 2026-08-07 is a useful reference design even though Source Weaver uses `eframe`, not Tauri. Tauri documents an updater that can use an update server or static JSON, and its update signatures are mandatory: the updater needs a signature to verify that the update comes from a trusted source and this cannot be disabled. It also documents per-platform updater artifacts and `.sig` files.

GitHub REST API release documentation checked on 2026-08-07 documents a `Get the latest release` endpoint under Releases. Source Weaver can use GitHub Releases as the public discovery source for manual update checks and, later, for signed update metadata.

## Decision

Source Weaver uses a staged update approach:

1. **Manual release discovery remains supported.** Users or maintainers can still check GitHub Releases, verify `SHA256SUMS`, verify `SHA256SUMS.asc` when present, and run the installer or replace the portable archive manually.
2. **Current signed-metadata stage.** Source Weaver has a signed update-manifest verifier, a CLI `sourceweaver update check` command, and a desktop opt-in update panel. The manifest signature is Ed25519 over the canonical JSON payload. Unsigned manifests are refused.
3. **Current download-only stage.** CLI and desktop download paths verify the signed manifest and artifact SHA-256 before writing an artifact. The `--install --confirm-install` CLI path and the desktop install-handoff button stop after verified download and tell the user to run the artifact manually.
4. **Future installer execution stage.** Any automatic installer launch or executable replacement remains out of scope until real release signing credentials, platform installer validation, rollback recovery, and desktop smoke evidence are recorded.

No automatic installer execution is present in the current implementation.

## Channels

Source Weaver supports these planned channels:

| Channel | Source | Intended audience | Auto-install eligibility |
| --- | --- | --- | --- |
| `stable` | non-prerelease `v*` GitHub Releases | normal users | eligible after required signing is enabled |
| `preview` | prerelease GitHub Releases | testers | check/download only until separate preview signing policy exists |
| `manual` | user-supplied artifact path or URL | maintainers and offline users | never auto-install without local verification |

The default channel is `stable`. Preview updates must never replace stable installations unless the user opted into preview.

## Security requirements

Automatic update code must meet these requirements before it can install anything:

- Use HTTPS update metadata endpoints.
- Require signed update metadata or signed checksum manifests.
- Verify artifact SHA-256 against signed metadata before install.
- Verify Windows Authenticode signatures when Windows signing is configured.
- Verify OpenPGP detached signatures for `SHA256SUMS` when present.
- Refuse downgrades unless a rollback token or explicit user action allows one.
- Refuse channel switches unless the user explicitly opts in.
- Never replace the currently running executable directly from an unverified download.
- Keep the current version usable until the new installer or archive has completed verification.
- Preserve user data and project files outside the application install directory.
- Log update checks, downloaded version, verification result, and user decision without recording secrets.

## User consent

Update checks may be passive, but installs must be user-initiated until Source Weaver has a dedicated preference screen. The desktop app should present:

- current version;
- latest version;
- channel;
- release notes link;
- artifact type and size;
- signature/checksum status;
- clear **Download**, **Install**, **Skip**, and **Remind me later** choices.

The CLI defaults to read-only checks through `sourceweaver update check`. Downloads require `--download-dir`. Install handoff requires both `--install` and `--confirm-install`; even then Source Weaver does not execute an installer automatically.

The desktop update panel is opt-in. It requires a manifest path or HTTPS URL and an Ed25519 public key. The user must press **Check signed manifest**, **Download verified artifact**, or **Prepare install handoff**. Startup never performs an update network request.

## Rollback and downgrade rules

Rollback is a safety feature, not a silent downgrade system.

- Keep the previous installer or archive reference in release notes and update metadata.
- Permit rollback only after explicit user consent.
- Require the same signature and checksum verification as forward updates.
- Block rollback when metadata marks the current version as a required security baseline.
- Document manual downgrade steps for portable zip and per-user NSIS installs.

## Offline behavior

Source Weaver must remain fully usable offline.

- Failed update checks should become non-fatal warnings.
- The desktop app should avoid blocking startup on network calls.
- The CLI update check should exit nonzero on network failure, with a clear message.
- Offline users can download artifacts elsewhere and verify `SHA256SUMS` locally.
- Installers and portable archives remain available so managed or air-gapped systems can update manually.

## Manual update path

Until automatic updates are implemented, users should update manually.

### Check latest release

From a source checkout:

```bash
scripts/check-latest-release.sh v0.1.0
```

The helper queries GitHub Releases, prints the latest release tag, release URL, prerelease status, and asset names, and compares the optional current version by exact normalized tag. It does not download or install anything.

Without a checkout, open the GitHub Releases page and compare the current app version with the latest non-prerelease tag.

### Verify downloads

Download the artifact for your platform and `SHA256SUMS`. When `SHA256SUMS.asc` exists, import the published release public key and verify the manifest signature:

```bash
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum -c SHA256SUMS
```

On Windows, PowerShell users can still inspect Authenticode status when signing is configured:

```powershell
Get-AuthenticodeSignature .\sourceweaver-vX.Y.Z-windows-x86_64-setup.exe | Format-List
```

### Install manually

Windows setup installer:

```powershell
.\sourceweaver-vX.Y.Z-windows-x86_64-setup.exe
```

Windows portable zip:

1. Close Source Weaver.
2. Extract the new zip into a new directory.
3. Run `sourceweaver.exe --help` or `sourceweaver-desktop.exe`.
4. Replace shortcuts only after verifying the new directory works.

Linux AppImage:

```bash
chmod +x sourceweaver-vX.Y.Z-linux-x86_64.AppImage
./sourceweaver-vX.Y.Z-linux-x86_64.AppImage
```

Linux tarball:

```bash
tar -xzf sourceweaver-vX.Y.Z-linux-x86_64.tar.gz
./sourceweaver-vX.Y.Z-linux-x86_64/sourceweaver --help
```

## Implemented signed-update design

The current implementation includes:

1. Ed25519-signed update manifests in `sourceweaver_core::update`.
2. Release-workflow manifest generation through `sourceweaver update manifest` when `SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64` is configured.
3. No unsigned update manifest publication. When the update signing key is absent, the workflow prints a refusal message and publishes no update metadata.
4. Manifest signature verification tests with fixed non-release test keys.
5. Corrupt-artifact and wrong-signature rejection tests.
6. A CLI `sourceweaver update check` command that reads signed metadata and reports update status.
7. Desktop opt-in update UI with release notes link and no startup network request.
8. Download-only support with checksum verification before writing the artifact.
9. Explicit install handoff confirmation. Source Weaver does not execute installers automatically.
10. Downgrade and channel-switch blocking rules.

Run the dedicated validation script with:

```bash
scripts/validate-signed-update-support.sh /tmp/sourceweaver-signed-update-validation
```

The script generates a synthetic release artifact, signs an update manifest with a fixed test key, verifies check-only behavior, verifies a download/install-handoff path, rejects a wrong manifest signature, rejects a corrupt artifact, and confirms rejected downloads are not written.

## Remaining future work

- Configure real release signing credentials and publish the update public key before enabling stable public update checks by default.
- Add platform installer launch only after Windows/Linux release signing, rollback recovery, and desktop smoke evidence are complete.
- Add optional local HTTP fixture tests if future network client behavior becomes more complex.
- Add Windows installer handoff tests on `windows-latest` without silent uncontrolled install.
- Record manual desktop UI smoke evidence before claiming end-to-end interactive update support for a public release.

## Current limitation

Source Weaver has signed update metadata verification, explicit check/download commands, and a desktop opt-in update panel. It does not have background update checks, silent installation, automatic installer execution, executable replacement, or automatic rollback.
