# Application update strategy

Source Weaver does not enable automatic self-updates yet. The safe current path is manual update discovery, artifact verification, and user-initiated install or replacement.

Automatic updates should be implemented only after signed release artifacts are enforced for the update channel. `docs/code-signing.md` defines the signing hooks and required secrets, but the current repository state has no real Windows code-signing certificate or OpenPGP release key configured.

## Research notes

Tauri's updater documentation checked on 2026-08-07 is a useful reference design even though Source Weaver uses `eframe`, not Tauri. Tauri documents an updater that can use an update server or static JSON, and its update signatures are mandatory: the updater needs a signature to verify that the update comes from a trusted source and this cannot be disabled. It also documents per-platform updater artifacts and `.sig` files.

GitHub REST API release documentation checked on 2026-08-07 documents a `Get the latest release` endpoint under Releases. Source Weaver can use GitHub Releases as the public discovery source for manual update checks and, later, for signed update metadata.

## Decision

Source Weaver will use a staged update approach:

1. **Current stage: manual update checks only.** Users or maintainers check GitHub Releases, verify `SHA256SUMS`, verify `SHA256SUMS.asc` when present, and run the installer or replace the portable archive manually.
2. **Next stage: signed update manifest.** A tag release publishes a machine-readable update manifest that includes version, channel, artifact URLs, SHA-256 digests, release notes URL, required minimum version if needed, and detached signatures.
3. **Final stage: opt-in automatic download/install.** The desktop app checks the update manifest after user consent, verifies signatures and hashes, prompts before install, and launches the platform installer or opens the downloaded artifact location. CLI support remains check-only unless an explicit `--download` or `--install` flag is added later.

No automatic installer execution should be added before signed update metadata is enforced.

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

The CLI should default to read-only checks. Future CLI download/install commands must require explicit flags such as `--download` or `--install`.

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

## Future implementation design

A future auto-update implementation should add these pieces in order:

1. Required release signing for stable releases.
2. A generated update manifest in the release workflow.
3. Manifest signature verification tests with fixed test keys.
4. A CLI `update check` command that reads signed metadata and reports update status.
5. Desktop UI for opt-in update checks and release notes.
6. Download-only support with checksum/signature verification.
7. Platform install handoff after explicit confirmation.
8. Rollback documentation and tests.

The first implementation should avoid background installation. It should open the installer or downloaded artifact location after verification and let the platform installer perform changes.

## Validation checklist for future auto-update code

- Unit tests for version comparison, channel filtering, manifest parsing, and downgrade blocking.
- Integration tests using a local HTTP fixture server for metadata and artifacts.
- Signature verification tests with generated test keys that are not release keys.
- Corrupt artifact tests that must fail checksum verification.
- Wrong-signature tests that must fail before download/install.
- Offline tests that prove startup is not blocked.
- Windows installer handoff tests on `windows-latest` without silent uncontrolled install.
- Manual desktop UI smoke evidence before claiming interactive update support.

## Current limitation

Source Weaver currently has a manual update check helper and documented update policy. It does not have in-app automatic update checks, automatic downloads, background installation, or automatic rollback.
