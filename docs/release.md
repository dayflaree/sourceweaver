# Release process

Source Weaver uses semantic version tags and GitHub Actions to build release artifacts.

## Versioning

Use `vMAJOR.MINOR.PATCH` tags, for example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Until the project reaches `v1.0.0`, breaking workflow or output changes may happen in minor versions. Patch versions should be bug fixes, documentation-only changes, or packaging fixes.

## Automated release workflow

Pushing a `v*` tag runs `.github/workflows/desktop-builds.yml`.

The workflow:

1. Builds Linux release binaries on `ubuntu-latest`.
2. Packages Linux CLI/desktop artifacts into `sourceweaver-<tag>-linux-x86_64.tar.gz`.
3. Packages Linux CLI/desktop artifacts into `sourceweaver-<tag>-linux-x86_64.AppImage`.
4. Builds Windows release binaries on `windows-latest`.
5. Packages Windows CLI/desktop artifacts into `sourceweaver-<tag>-windows-x86_64.zip`.
6. Installs NSIS, packages the Windows setup executable into `sourceweaver-<tag>-windows-x86_64-setup.exe`, and validates silent install/uninstall.
7. Uploads all packages as workflow artifacts.
8. Creates `SHA256SUMS` for the release artifacts and signs it as `SHA256SUMS.asc` when an OpenPGP release key is configured.
9. Creates or updates a GitHub Release for the tag.
10. Uploads the tarball, AppImage, zip, Windows setup executable, `SHA256SUMS`, and optional `SHA256SUMS.asc` to the GitHub Release.

## Changelog

Update `CHANGELOG.md` before tagging. The release workflow uses the repository changelog as the release-note body, so the newest entry should summarize user-facing changes, compatibility notes, and known limitations.

## Manual dry run

The packaging workflow also supports `workflow_dispatch`. Manual dispatch builds and uploads workflow artifacts without requiring a local Linux/Windows packaging environment.

Local Linux tarball dry run:

```bash
scripts/package-linux.sh v0.1.0-local
```

Local Linux AppDir/AppImage dry run:

```bash
scripts/package-appimage.sh v0.1.0-local --appdir-only
APPIMAGETOOL=/path/to/appimagetool-x86_64.AppImage scripts/package-appimage.sh v0.1.0-local
```

Local Windows package dry run from PowerShell with NSIS installed:

```powershell
scripts\package-windows.ps1 -Version v0.1.0-local -RequireInstaller
scripts\validate-windows-installer.ps1
```

Local Windows portable-zip-only dry run:

```powershell
scripts\package-windows.ps1 -Version v0.1.0-local -SkipInstaller
```

## Release checklist

1. Confirm `cargo fmt --check`, `cargo test --workspace`, and `cargo build --workspace` pass.
2. Confirm CLI job-runner dry-run JSON validation passes.
3. Confirm `sourceweaver validate` can validate the fixture merged VMF with the sample VBSP log.
4. Update `CHANGELOG.md`.
5. Review `docs/code-signing.md` and confirm whether signing secrets are configured for this release.
6. Push a `vMAJOR.MINOR.PATCH` tag.
7. Wait for Linux and Windows release jobs to pass.
8. Confirm the Windows job reports setup install/uninstall validation.
9. Confirm `SHA256SUMS` was generated and, when configured, `SHA256SUMS.asc` verifies with the release public key.
10. Download and smoke-test the published release archives and installer when access to the relevant OS is available.
11. Run `scripts/check-latest-release.sh <previous-version>` after publishing to verify the manual update-check path sees the new release.

## Current packaging limitations

- Linux AppImage packaging is wired into the release workflow, but clean Linux GUI smoke evidence must be recorded per release.
- Windows NSIS installer packaging is wired into the release workflow, but interactive GUI smoke evidence outside silent CI install/uninstall must be recorded per release.
- Release artifacts are unsigned unless Windows code-signing and OpenPGP release-signing secrets are configured; see `docs/code-signing.md`.
- Automatic updates are not enabled; use the manual update path in `docs/update-strategy.md` until signed update metadata is required.
- Real Hammer/VBSP/VVIS/VRAD/game-runtime validation still requires a user-provided Source tool installation or captured compile logs. Record completed real-tool evidence in `docs/source-compiler-smoke-test-matrix.md` before making release claims.
