# Changelog

## Unreleased

### Added

- Linux release tarball packaging with desktop entry and SVG icon.
- Windows release zip packaging with desktop and CLI executables plus icon asset.
- Tag-driven GitHub release workflow for `v*` versions.
- Portable Source-tool validation and captured VBSP log parsing.
- Source-colored merged previews, landmark markers, offset arrows, deletion overlays, and preview click selection.
- Desktop project save/load, drag-and-drop import, recent files, FGD metadata loading, and cleanup confirmation.
- Campaign transition detection, campaign order suggestions, and landmark-pair suggestions.
- Fixture/golden regression coverage for parser, merge, prune, preview, and automation reports.
- First-class BSPSource CLI/jar decompile runner with version/provenance reporting while keeping generic wrapper support.
- Optional BSP content packing command for user-provided `bspzip`-compatible tools, generated file lists, and JSON reports.
- Compile profile create/validate/discover command plus Linux/Wine/Proton wrapper examples and setup docs.
- Optional desktop compile panel that launches the CLI compile pipeline after export or on demand without blocking the UI.

### Changed

- Merge translation now handles displacement `startposition`, texture-axis offsets, and known VMF ID reference remapping.
- Editor metadata merge behavior is documented and test-covered.

### Known limitations

- Linux releases are tarballs rather than AppImages.
- Windows releases are zip archives rather than installers.
- Release artifacts are not code-signed.
- Real Hammer/VBSP validation requires installed Source tooling or captured compile logs.
