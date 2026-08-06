# Roadmap

## Milestone 1: VMF foundation

- Parse and write VMF KeyValues.
- Inspect all entity classnames.
- Detect core brush roles.
- Delete by classname, targetname, and brush role.
- Merge VMFs with landmark alignment.
- Preserve skybox solids during merges.
- Add CLI validation commands.

Status: implemented.

## Milestone 2: Desktop UI

- Add cross-platform desktop shell.
- Add VMF file picker and selected-map list.
- Add base-map selector.
- Add landmark and output controls.
- Add entity table with classname, targetname, origin, solids, and roles.
- Add classname summary table.
- Add deletion filters and preview.
- Add cleaned VMF export.
- Add merge/export action.
- Add status log.
- Add Hammer-style 2D orthographic preview.
- Add merged-output preview before export.
- Add preview pan and zoom.
- Add role-colored brush overlays and entity origin markers.
- Add landmark discovery dropdown and missing/duplicate landmark warnings.
- Add VMF integrity checks before writing merged output.
- Add checkbox bulk selection to the entity table.
- Add search, filtering, and sortable columns for entities and classnames.
- Add safe deletion modes for brush entities and world solids.
- Add deletion presets and protected critical entities.
- Add desktop project save/load using CLI-compatible TOML job files.
- Detect `trigger_changelevel` campaign transition entities.
- Add portable Source-tool validation and VBSP compile-log parsing.
- Color merged-preview geometry by source VMF.
- Render landmark markers, selected-landmark labels, and offset arrows in preview.
- Suggest campaign order and landmark pairs from transition entities.
- Add visual deletion-rule overlays in preview.
- Add preview click selection and entity-table synchronization.
- Add pending cleanup review, undo, and confirmation before destructive export.
- Expand VMF fixtures and golden output regression tests.
- Harden displacement `startposition` translation with fixture coverage.
- Adjust texture-axis offsets during brush translation.
- Remap known VMF ID reference fields during merge renumbering.
- Document and test editor metadata merge policy.
- Add built-in, inferred, and FGD-loaded entity metadata.
- Improve desktop usability with drag-and-drop, recents, progress, error dialogs, theme toggle, and preview sizing.
- Package Linux and Windows desktop releases from tag-driven GitHub Actions workflows.
- Automate GitHub Releases with changelog-backed release notes.
- Reconstruct convex brush face polygons for more accurate 2D preview rendering.
- Add lightweight 3D isometric preview with yaw/pitch camera controls.
- Add optional VBSP/VVIS/VRAD compile pipeline with captured logs and JSON reports.
- Document VMF-first BSP decompile import recommendation and follow-up scope.
- Validate two public real VMFs end-to-end through inspect, merge, validation, and compile-pipeline reporting.
- Add user-provided BSP decompiler wrapper and desktop BSP-derived VMF import warnings.
- Harden external compiler/decompiler execution with timeouts, large-log-safe capture, stricter success parsing, Clippy CI, and 3D preview fallback hit testing.

Status: first native Linux/Windows UI, desktop drag/drop usability, in-session recents, parse progress, error dialogs, theme toggle, adjustable preview sizing, reconstructed 2D face polygons, 3D isometric preview, 2D preview slices, merged-output preview, source-colored preview provenance, landmark markers/offset arrows, visual deletion overlays, preview/table click synchronization, pending cleanup confirmation, landmark warnings, integrity warnings, entity table selection, inspection-table filtering/sorting, entity metadata, safe deletion modes, deletion presets, project save/load, transition detection, campaign-order suggestions, compiler-log validation, optional compile pipeline, external-tool timeout hardening, displacement translation hardening, texture-axis translation, ID-reference remapping, editor metadata policy, BSP import recommendation/wrapper, fixture/golden regression tests, public real-VMF validation, desktop release packages, and tag-driven GitHub Releases implemented.

## Milestone 3: Source/Hammer validation

- Expand VMF integrity checks with game-specific validation rules.
- Detect duplicate targetnames created by merge.
- Detect potential Hammer ID conflicts.
- Add optional compile-tool integration where available.

## Milestone 4: Campaign conveniences

- Read transition entities such as `trigger_changelevel`.
- Suggest landmark pairings automatically.
- Detect likely map adjacency.
- Preserve or rewrite changelevel-related entities according to user rules.
- Add rule presets for common cleanup tasks.

## Milestone 5: Advanced editing

- FGD-backed property labels.
- Saved deletion presets.
- Batch mode for many campaign maps.
- Interactive preview selection.
- Full 3D textured preview.
- Packaging for Linux and Windows releases.
