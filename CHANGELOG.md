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
- Real Source compiler smoke-test matrix documentation for release evidence and manual validation.
- Desktop BSP decompile/import panel that runs the CLI BSPSource workflow, validates output VMFs, imports them, and marks them as BSP-derived.
- Optional model tooling slice with native MDL header inspection, user-provided StudioMDL-compatible compile reports, and Crowbar research/licensing docs.
- Opt-in game validation rule-set model with an initial portable HL2 single-player profile exposed in CLI JSON/text reports and the desktop integrity panel.
- Entity semantic validation for duplicate targetnames and missing common target references, reported separately in CLI JSON/text output and the desktop integrity panel.
- Centralized VMF ID-reference remap policy plus integrity warnings for unsupported suspected ID-reference fields.
- VMF complexity heuristic summary for entities, brush solids, sides/faces, displacements, and overlays in CLI JSON/text output and desktop validation UI.
- Changelevel preservation, disable, delete, and internal-rewrite policies for CLI merge, CLI jobs, and desktop merge workflows.
- Configurable transition cleanup scope and external preserve selectors with dry-run JSON diffs and desktop project persistence.
- Campaign adjacency graph report with high-confidence trigger edges and separate shared-landmark/filename-sequence heuristic edges.
- Multi-step campaign batch plan runner with per-step reports, summary artifact links, and dry-run planning.
- Desktop BSP packing panel with user-configured packer tool, BSP paths, asset/filelist inputs, reports, output tails, and optional pack-after-compile.
- Desktop model tooling panel for MDL metadata inspection and background StudioMDL-compatible model compile integrations.
- Desktop compile profile wizard for creating, validating, and discovering user-provided VBSP/VVIS/VRAD profiles.
- Custom deletion preset TOML format with CLI job application and desktop save/export/load/import controls.
- FGD-backed entity property metadata parsing with labels, descriptions, defaults, choices/flags, CLI inspect JSON/text output, and desktop property tooltips.
- Desktop material-aware preview colors reconstructed brush faces by VMF material names, scanned user-selected roots, missing-material fallbacks, and fixed tool-texture colors.
- Managed BSPSource manifest, policy, cache-path, checksum-verify, and explicit download helper with pinned v1.4.8 SHA-256 assets.
- BSPSource decompile-quality parser with categorized unsupported-lump, skipped-data, quality-risk, tool-error, warning, and non-fatal configuration-noise reporting in CLI JSON and desktop UI.
- BSPSource argument presets for embedded asset extraction, smart-unpack auditing, manual areaportal mapping, and tool/cubemap texture-fix toggles, exposed in CLI and desktop while keeping raw tool args.
- Synthetic legally redistributable BSP-derived fixture set with manifest/checksums and CI fake-wrapper import regression coverage.
- External BSP decompiler preset registry with VMEX legacy research, do-not-bundle policy, and generic wrapper examples.
- Cubemap/buildcubemaps workflow planner with game-profile caveats, optional cfg helper output, JSON reports, and documented real-runtime validation boundary.
- VMF-driven BSP pack dependency discovery for common material, model, sound, script, particle, VMT texture, and model companion assets, integrated with CLI pack reports.
- BSPZIP/BSPZIP++ context profiles, wrapper examples, and pack CLI fields for tool working directories, LD_LIBRARY_PATH, and explicit wrapper-compatible `-game` forwarding.
- External `model-decompile` headless-wrapper runner with placeholder argument expansion, log capture, output discovery, structured reports, and documented Crowbar bundling/validation boundary.
- Native `model-inspect` bodypart/model/mesh metadata parsing for supported Source MDL layouts with version-aware warnings and synthetic fixture coverage.
- Native `model-inspect` local animation and sequence descriptor metadata parsing for supported Source MDL layouts, without deep animation-frame decoding.
- Native `model-inspect` material dependency parsing from Source MDL texture/material-directory tables, with optional asset-root resolution and missing/ambiguous reports.
- Native `model-inspect` VVD/VTX/PHY companion-file metadata probing with missing and checksum-mismatch reporting.
- Model source-output manifest workflow for classifying externally generated QC/QCI, SMD, DMX, VTA, and other decompile outputs without running proprietary tools.
- Model package manifest/copy workflow for MDL, VVD/VTX/PHY companions, and resolved material dependencies from user-selected asset roots.
- Metadata-only model preview reports and optional user-provided HLMV-compatible launch reporting with explicit external-tool boundaries.
- Steam Source tool discovery for compile profiles, including VBSP/VVIS/VRAD/BSPZIP/StudioMDL candidate details, confidence, and runtime caveats.
- Completed Proton-backed real VBSP++/VVIS++/VRAD++ smoke-test matrix row for Garry's Mod Source++ tools and adjusted compile-step reporting for quiet wrapper logs.
- Hammer/Hammer++ open/save validation workflow documentation with required evidence fields, saved-VMF diff checks, and sanitization rules.
- Runtime map-load validation workflow planning command and documentation for console/log evidence, missing assets, crashes, and gameplay smoke notes.
- Verified Proton compile setup documentation for the Garry's Mod Source++ real compiler row, including compatdata and evidence paths.
- Wine compile blocker documentation with command evidence showing no Wine runtime available in the current environment.
- Linux AppImage packaging script and release workflow integration, with AppDir validation and packaging documentation.
- Windows NSIS installer packaging script, CI install/uninstall validation, release workflow integration, and packaging documentation while retaining portable zip releases.
- Release checksum manifest generation, optional OpenPGP checksum signing, Windows Authenticode signing hooks, and code-signing policy documentation.
- Manual latest-release check helper and documented auto-update strategy covering signing, channels, rollback, consent, and offline behavior.
- Third-party tool redistribution policy with never-bundled, user-provided-only, managed-download, and redistributable-candidate categories plus review gates for future managed downloads.
- Real StudioMDL++ model-compile validation row using a Source Weaver-authored synthetic QC/SMD fixture and Wine wrapper evidence.
- Rust dependency vulnerability audit gate with `cargo-audit`, CI enforcement, release checklist coverage, and documented accepted transitive warnings.
- Third-party policy review issue template and CI check that enforce completed `third_party_policy_review` records for managed downloads and redistributable candidates.
- High-risk map-case validation script and evidence matrix covering displacement-heavy, textured-material, nested/hidden group, `func_instance`, and large campaign complexity boundaries.

### Changed

- Merge translation now handles displacement `startposition`, texture-axis offsets, and known VMF ID reference remapping.
- Editor metadata merge behavior is documented and test-covered.

### Known limitations

- Linux AppImage GUI smoke evidence must be recorded per release on a clean Linux environment.
- Windows setup installers need interactive GUI smoke evidence outside silent CI install/uninstall.
- Release artifacts are unsigned unless Windows code-signing and OpenPGP release-signing secrets are configured.
- Automatic updates are intentionally disabled until signed update metadata and release signing enforcement exist.
- Third-party tools and assets stay unbundled unless the redistribution policy review records rights, provenance, attribution, checksum, update, and removal decisions.
- Real StudioMDL++ validation produced local evidence artifacts only; generated model outputs are not committed or redistributed.
- Real Hammer/VBSP validation requires installed Source tooling or captured compile logs.
