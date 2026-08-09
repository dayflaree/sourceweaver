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

Pushing a `v*` tag runs `.github/workflows/desktop-builds.yml`. The workflow resolves a release mode before packaging or publishing:

- Preview mode is selected for manual `workflow_dispatch` runs by default and for tags containing `-preview`, `-alpha`, `-beta`, or `-rc`.
- Final mode is selected for plain `vMAJOR.MINOR.PATCH` tags and can also be selected during `workflow_dispatch` as a dry-run policy check.
- Preview mode publishes GitHub Releases with `prerelease: true` and `make_latest: false`.
- Final mode publishes GitHub Releases with `prerelease: false` and `make_latest: true`.
- Final mode fails before packaging when required production signing secret or variable names are absent: `SOURCEWEAVER_WINDOWS_SIGNING_PFX_BASE64`, `SOURCEWEAVER_WINDOWS_SIGNING_PFX_PASSWORD`, `SOURCEWEAVER_WINDOWS_TIMESTAMP_URL`, `SOURCEWEAVER_WINDOWS_SIGNTOOL`, `SOURCEWEAVER_GPG_PRIVATE_KEY_BASE64`, and `SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64`.
- Final mode passes `-RequireSigning` to the Windows packaging script, sets `SOURCEWEAVER_REQUIRE_RELEASE_SIGNATURES=1` for checksum signing, and runs signed update metadata generation as a required step.

Validate the final-mode fail-closed policy locally without secret values:

```bash
scripts/validate-final-release-policy.sh
```

The validation proves preview mode permits absent credentials, final mode refuses absent credential names, the OpenPGP checksum-signing script fails when required key material is absent, and the workflow keeps final release `prerelease` / `make_latest` settings wired to the resolved mode.


The workflow:

1. Builds Linux release binaries on `ubuntu-latest`.
2. Packages Linux CLI/desktop artifacts into `sourceweaver-<tag>-linux-x86_64.tar.gz`.
3. Packages Linux CLI/desktop artifacts into `sourceweaver-<tag>-linux-x86_64.AppImage`.
4. Builds Windows release binaries on `windows-latest`.
5. Packages Windows CLI/desktop artifacts into `sourceweaver-<tag>-windows-x86_64.zip`.
6. Installs NSIS, packages the Windows setup executable into `sourceweaver-<tag>-windows-x86_64-setup.exe`, and validates silent install/uninstall.
7. Uploads all packages as workflow artifacts.
8. Runs a provenance job that generates `sourceweaver-sbom.cdx.json`, creates `SHA256SUMS`, uploads those files as provenance workflow artifacts, and generates GitHub artifact attestations for the package, checksum, and SBOM files.
9. On tags, signs `SHA256SUMS` as `SHA256SUMS.asc` when an OpenPGP release key is configured.
10. On tags, creates signed update metadata only when `SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64` is configured; unsigned update metadata is refused.
11. On tags, generates final GitHub artifact attestations for all files in `target/release-artifacts/*`, including optional signatures and update metadata when present.
12. Creates or updates a GitHub Release for the tag.
13. Uploads the tarball, AppImage, zip, Windows setup executable, `SHA256SUMS`, `sourceweaver-sbom.cdx.json`, optional `SHA256SUMS.asc`, and optional signed `sourceweaver-update-manifest.json` to the GitHub Release.

## Changelog

Update `CHANGELOG.md` before tagging. The release workflow uses the repository changelog as the release-note body, so the newest entry should summarize user-facing changes, compatibility notes, and known limitations.

## Manual dry run

The packaging workflow also supports `workflow_dispatch`. Manual dispatch builds and uploads package plus provenance workflow artifacts without requiring a local Linux/Windows packaging environment. The tag-only publish job is skipped, so no GitHub Release is created during a manual dry run. Use the `release_mode` input to run preview packaging or to dry-run final-mode signing policy checks before creating a plain final tag.

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
2. Confirm `cargo audit` output is reviewed, then run `scripts/cargo-audit-final-release.sh` so unapproved warnings fail while the two documented unmaintained advisory IDs stay narrowly allowed.
3. Confirm CLI job-runner dry-run JSON validation passes.
4. Confirm `sourceweaver validate` can validate the fixture merged VMF with the sample VBSP log.
5. Confirm `python3 scripts/check-validation-claims.py --self-test` and `python3 scripts/check-validation-claims.py` pass before release notes make compatibility or signing claims.
6. Update `CHANGELOG.md` and include the preview limitation lines `Hammer/Hammer++ open/save: not certified.` and `Native Windows Source compiler execution: not certified.` unless completed evidence rows exist in `docs/compatibility-matrix.md`.
7. Review `docs/code-signing.md` and `docs/provenance-sbom.md`, complete the release-note signing/provenance template, and confirm whether signing secrets are configured for this release. Unsigned previews are allowed only when release notes explicitly say artifacts from that run are unsigned and list OpenPGP/update-manifest status.
8. Confirm #143 remains open when production signing credentials are absent.
9. Generate or verify `sourceweaver-sbom.cdx.json` and GitHub artifact attestations for the release artifacts.
10. Push a `vMAJOR.MINOR.PATCH-preview.N` tag for an unsigned preview posture, or a plain `vMAJOR.MINOR.PATCH` tag only after final-mode signing credentials and variables are configured.
11. Wait for Linux, Windows, provenance, and release jobs to pass.
12. Confirm the Windows job reports setup install/uninstall validation.
13. Confirm `SHA256SUMS` and `sourceweaver-sbom.cdx.json` were generated, `scripts/generate-release-sbom.py --validate-only sourceweaver-sbom.cdx.json` passes, and, when configured, `SHA256SUMS.asc` verifies with the release public key.
14. Confirm `gh attestation verify <artifact> --repo dayflaree/sourceweaver` passes for published artifacts when network access and repository permissions allow it.
15. When `SOURCEWEAVER_UPDATE_SIGNING_KEY_BASE64` is configured, confirm `sourceweaver-update-manifest.json` exists and `sourceweaver update check --manifest sourceweaver-update-manifest.json --public-key <published-key>` verifies it. When the key is absent, confirm no unsigned update manifest was published.
16. Download and smoke-test the published release archives and installer when access to the relevant OS is available.
17. Run `scripts/check-latest-release.sh <previous-version>` after publishing to verify the manual update-check path sees the new release.

## Current packaging and validation limitations

- Linux AppImage packaging is wired into the release workflow, but clean Linux GUI smoke evidence must be recorded per release.
- Windows NSIS installer packaging is wired into the release workflow, but interactive GUI smoke evidence outside silent CI install/uninstall must be recorded per release.
- Production release signing is absent unless the specific release run records configured Windows Authenticode, OpenPGP, and/or update-signing credentials; see `docs/code-signing.md`. Unsigned preview releases are acceptable only with explicit unsigned/signing/provenance release-note status. GitHub artifact attestations and SBOMs are provenance aids, not production signing substitutes.
- Signed update checks and verified download/install handoff are implemented. Automatic installer execution, executable replacement, silent install, persistent update preferences, and rollback are not enabled; use the manual update path and signed-metadata flow in `docs/update-strategy.md` and `docs/update-install-roadmap.md`.
- Hammer/Hammer++ open/save compatibility is `not validated` in `docs/compatibility-matrix.md` and is not certified until a real Hammer/Hammer++ executable opens and saves generated VMFs and the saved output is diffed and recorded.
- Native Windows Source compiler execution is `not validated` in `docs/source-compiler-smoke-test-matrix.md` Row B and is not certified until real native Windows VBSP/VVIS/VRAD execution evidence is recorded. CI Windows Rust build/test coverage, Windows release packaging, and Proton/Wine wrapper rows must stay labeled as separate evidence.
- Successful game-runtime map load is not certified. Existing runtime evidence records real launch failure, not a playable map-load pass.
- Rendered HLMV/HLMV++ model preview is not certified. Existing HLMV evidence records external launch plumbing and failure before a rendered window opened.
- Real Hammer/VBSP/VVIS/VRAD/game-runtime validation still requires user-provided Source tool installations or captured logs. Record completed real-tool evidence in `docs/source-compiler-smoke-test-matrix.md`, `docs/runtime-map-load-validation.md`, or `docs/model-tooling.md` before making release claims. Any managed third-party download or bundled third-party asset must pass `docs/third-party-redistribution-policy.md` before release.
