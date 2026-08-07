# Custom deletion presets

Source Weaver supports user-saved deletion presets as transparent TOML files. The format is compatible with CLI jobs and desktop projects: the preset contains metadata plus a `[delete]` table that uses the same cleanup fields as `sourceweaver run --job` and saved desktop project files.

## Format

```toml
name = "Remove prop_detail"
description = "Drops prop_detail entities while preserving critical gameplay entities."

[delete]
classnames = ["prop_detail"]
targetnames = []
roles = []
all_entities = false
brush_entity_mode = "whole-entity"
protect_critical_entities = true
```

Supported `[delete]` fields:

- `classnames`: entity classnames to remove;
- `targetnames`: entity targetnames to remove;
- `roles`: brush role filters such as trigger, skip, hint, tool, nodraw, and clip roles supported by Source Weaver;
- `all_entities`: remove all non-protected top-level entities;
- `brush_entity_mode`: `whole-entity` or `matching-solids`;
- `protect_critical_entities`: keep critical transition/player/logic entities protected unless explicitly disabled.

## CLI jobs

Add `delete_preset` to a job. The path is resolved relative to the job TOML file:

```toml
base = "../fixtures/deletion_preset.vmf"
output = "../../target/test-output/deletion_preset.vmf"
delete_preset = "../presets/remove-prop-detail.toml"
dry_run = true

[delete]
protect_critical_entities = true
```

The job report includes `deletion_preset` with the resolved preset path and the expanded `deletion` criteria/report. Inline `[delete]` values can add to the preset criteria and can override safety/mode settings. Preview the JSON report before writing destructive cleanup outputs.

## Desktop import/export

The desktop **Bulk deletion rules** panel includes **Custom deletion presets**:

- **Name** and **Description** fields become preset metadata;
- **Path** chooses where to save/export or load/import a preset TOML file;
- **Save/export current preset** writes the current cleanup controls to TOML;
- **Load/import preset** reads TOML, applies its `[delete]` criteria to the cleanup controls, and clears any stale pending cleanup review.

After loading or saving a preset, use **Preview deletion** to verify removal counts before exporting a cleaned VMF or merged output.

## Fixtures and tests

- `tests/presets/remove-prop-detail.toml` demonstrates the shared preset format.
- `tests/jobs/deletion-preset.toml` applies that preset in dry-run mode.
- `tests/fixtures/deletion_preset.vmf` contains a removable `prop_detail` plus a protected player start.
- CLI regression tests assert that preset application is reported and removes the expected entity.
