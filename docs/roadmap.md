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

Status: first native Linux/Windows UI, 2D preview slices, merged-output preview, source-colored preview provenance, landmark markers/offset arrows, landmark warnings, integrity warnings, entity table selection, inspection-table filtering/sorting, safe deletion modes, deletion presets, project save/load, transition detection, campaign-order suggestions, and compiler-log validation implemented.

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
