# Automatic update install, rollback, and preferences roadmap

Source Weaver's update system is currently **check/download/install-handoff only**. It verifies signed update metadata, verifies artifact hashes, downloads artifacts on explicit request, and can prepare a manual install handoff after explicit confirmation. It does not execute installers, replace running executables, persist full updater preferences, or perform rollback.

## Decision

Automatic installer execution and executable replacement are out of current scope for both CLI and desktop.

Current scope:

```text
update_install_scope = "signed-check-download-manual-handoff"
automatic_installer_execution = false
executable_replacement = false
automatic_rollback = false
preference_persistence = "session-only/update-panel fields only"
```

## Platform and install-mode matrix

| Platform/artifact | Current mode | Future install mode | Current boundary |
| --- | --- | --- | --- |
| Windows NSIS setup executable | signed manifest check, verified download, manual handoff | explicit installer launch after Authenticode and installer smoke evidence | Source Weaver does not execute the setup executable. |
| Windows portable zip | signed manifest check, verified download, manual handoff | close app, extract to staged directory, verify CLI/desktop smoke, switch shortcut by explicit user action | Source Weaver does not overwrite the current install directory. |
| Linux AppImage | signed manifest check, verified download, manual handoff | stage new AppImage next to old file and ask user to replace launcher/symlink | Source Weaver does not replace the running AppImage. |
| Linux tarball | signed manifest check, verified download, manual handoff | extract to versioned directory, verify CLI smoke, ask user to update launcher/symlink | Source Weaver does not overwrite the current extracted tree. |
| GitHub prerelease/preview channel | check/download/manual handoff only | no automatic install until separate preview-signing policy and opt-in are complete | Preview artifacts must not replace stable installs unless the user opted into preview. |
| Offline/local artifact | local checksum/signature verification and manual install | no automatic install | Source Weaver cannot infer trust from local files without explicit signed metadata and user action. |

## Automatic-install readiness policy

`sourceweaver_core::update::evaluate_automatic_install_readiness` is a pure policy helper used to keep future updater states testable. It currently blocks automatic install unless all required gates are true:

- signed update manifest verified;
- artifact SHA-256 verified;
- user explicitly confirmed install;
- platform installer/package validation evidence exists;
- production signing/trust evidence exists;
- rollback plan exists;
- update preferences are persisted;
- the requested channel is selected;
- downgrade has explicit rollback consent.

The helper does not launch processes, move files, write preferences, or install anything. It exists so future CLI/desktop code can share a tested safety policy before any installer-runner work starts.

## Required user approval

No update installer may execute without explicit user approval. Future UI must show:

- current version and candidate version;
- channel and target artifact;
- release notes link;
- manifest signature status;
- artifact SHA-256 status;
- platform signature/trust status when applicable;
- installation location and backup/rollback location;
- consequences of downgrade or channel switch;
- **Install now** and **Cancel** actions.

CLI future shape should require an unmistakable two-flag confirmation, for example:

```bash
sourceweaver update install \
  --manifest sourceweaver-update-manifest.json \
  --public-key <published-key> \
  --download-dir ~/.cache/sourceweaver/updates \
  --confirm-install \
  --acknowledge-installer-execution
```

The existing `sourceweaver update check --install --confirm-install` remains manual handoff only.

## Rollback and downgrade policy

Rollback is a recovery path. It is not a silent downgrade mechanism.

A future rollback implementation must:

- retain the previous working artifact or installer reference before switching;
- verify rollback artifacts with the same signed metadata and SHA-256 requirements as forward updates;
- block rollback below `minimum_required_version` unless a signed emergency policy permits it;
- require explicit user consent for downgrade/rollback;
- record before/after versions, artifact names, hashes, exit status, and recovery steps;
- keep user data and project files outside the replaceable app directory;
- restore the previous version or provide manual recovery instructions when installer execution fails.

## Preference persistence plan

Current desktop fields are session state. Future preference persistence should store only non-secret updater preferences:

- preferred channel: `stable` or `preview`;
- update-check cadence: `never`, `manual`, or `startup-notify`;
- last skipped version;
- last successful check timestamp;
- last downloaded artifact path;
- user consent for preview channel;
- no private keys, signing keys, passwords, or proprietary paths beyond a user-selected download directory.

Startup network checks should remain disabled until persistence and consent UI are implemented. A future `startup-notify` mode must be non-blocking and must never install without a separate explicit click.

## Failure and offline behavior

Future automatic-install work must handle:

- offline metadata fetch failure as a non-fatal desktop warning;
- CLI network failure as a clear nonzero exit;
- partial downloads with `.partial` cleanup;
- checksum/signature mismatch with rejected download deletion;
- installer exit code capture;
- timeout and process cleanup for installer launch;
- rollback instructions when the platform installer reports failure;
- redaction of private home, Steam, cache, and project paths in evidence.

## Validation required before release wording changes

Before any release can claim installer execution, executable replacement, persistent update preferences, or rollback, evidence must include:

```bash
cargo test --workspace
scripts/validate-signed-update-support.sh /tmp/sourceweaver-signed-update-validation
python3 scripts/check-validation-claims.py --self-test
python3 scripts/check-validation-claims.py
```

Additional future validation must include platform-specific installer dry runs, rollback failure simulations, preference file migration tests, and desktop UI smoke evidence. Windows installer execution evidence must stay on a Windows machine/runner. Linux replacement tests must avoid mutating the running developer checkout.

## External references checked

- Tauri v2 updater documentation, checked 2026-08-08: https://v2.tauri.app/plugin/updater/ . It documents an updater plugin for Windows/Linux/macOS, mandatory update signatures that cannot be disabled, platform-specific updater artifacts, and installer/update configuration. Source Weaver uses `eframe`, so this is design reference only.
- Microsoft Windows Installer rollback documentation, checked 2026-08-08: https://learn.microsoft.com/en-us/windows/win32/msi/rollback-installation . It says Windows Installer generates rollback scripts/files during installation and automatically performs rollback after unsuccessful installation by default. Source Weaver currently packages NSIS rather than MSI, so this is a rollback design reference only.
