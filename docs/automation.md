# Non-interactive automation

Source Weaver can be driven entirely from the command line with no desktop interaction. This is the preferred workflow for assistant-driven map stitching because every operation can be represented as a TOML job file and every run emits a JSON report.

## Create a job file

Generate a starter TOML file:

```bash
cargo run -p sourceweaver-cli -- job-template > sourceweaver-job.toml
```

Example job:

```toml
base = "base.vmf"
inputs = ["next.vmf", "another.vmf"]
output = "stitched.vmf"
landmark = "map_transition"
dry_run = false
report = "sourceweaver-report.json"

[delete]
classnames = ["prop_static"]
targetnames = ["cleanup_me"]
roles = ["trigger", "clip"]
all_entities = false
brush_entity_mode = "whole-entity"
protect_critical_entities = true
```

Relative paths are resolved from the directory containing the job file. This makes jobs portable inside project folders. The desktop app can save and load the same project/job shape: UI-created files include `base`, `inputs`, `output`, `landmark`, and `[delete]` fields that are compatible with `sourceweaver run --job` where possible.

## Run a job

```bash
cargo run -p sourceweaver-cli -- run --job sourceweaver-job.toml
```

The command prints a JSON report to stdout. If the job contains `report = "..."` or the command uses `--report`, the same report is written to disk. Reports include VMF integrity counts and issue details so missing common sections, duplicate IDs, invalid IDs, and world-block errors are visible to automation. Reports also include detected `trigger_changelevel` transitions with target map and landmark data.

```bash
cargo run -p sourceweaver-cli -- run \
  --job sourceweaver-job.toml \
  --report reports/latest.json
```

## Dry run

Use dry runs for inspection and planning without writing the output VMF:

```bash
cargo run -p sourceweaver-cli -- run --job sourceweaver-job.toml --dry-run
```

Dry-run mode still parses every VMF, applies deletion rules in memory, performs the merge in memory, runs integrity checks, and reports what would happen.

## Deletion safety

Job files must make brush-entity role behavior explicit with `delete.brush_entity_mode`:

- `whole-entity` preserves the original behavior: when a brush entity matches a selected brush role, the whole entity is removed.
- `matching-solids` keeps brush entities and removes only contained solids that match selected brush roles. When the selected role is `brush-entity`, all contained solids in brush entities are removed.

`delete.all_entities` removes all non-protected top-level entities and is used by world-only cleanup presets. `delete.protect_critical_entities` defaults to `true`. Protected classnames include transition/player/logic entities such as `info_landmark`, `trigger_changelevel`, `info_player_start`, `logic_auto`, and related control entities. Set it to `false` only when a job intentionally removes those entities.

## Clean a single VMF

A job with only `base` and no `inputs` performs a clean/prune operation instead of a merge:

```toml
base = "map.vmf"
output = "map_cleaned.vmf"
report = "map_cleaned-report.json"

[delete]
roles = ["trigger"]
```

## Assistant workflow

For assisted development, the operator can provide or identify VMF paths once. After that, the assistant can:

1. Generate a job file.
2. Run `sourceweaver run --job ... --dry-run`.
3. Read the JSON report.
4. Adjust deletion rules or landmark settings.
5. Run the final job.
6. Validate the output and report results.

No GUI clicks or manual command composition are required.
