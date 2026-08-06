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

Status: first native Linux/Windows UI slice implemented.

## Milestone 3: Source/Hammer validation

- Add VMF integrity checks.
- Detect missing landmarks before merge.
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
- Optional 2D/3D preview.
- Packaging for Linux and Windows releases.
