# Transition cleanup and preserve rules

Source Weaver can apply transition-aware cleanup rules to `trigger_changelevel` entities after a merge. These rules are portable VMF text edits only. They do not run Hammer, Hammer++, VBSP, VVIS, VRAD, a game runtime, or any external SDK tool.

## Cleanup scope

`changelevel_scope` narrows which transitions a cleanup policy touches:

| Scope | Behavior |
| --- | --- |
| `all` | Apply the selected changelevel policy to every top-level `trigger_changelevel`, unless an external preserve rule matches. |
| `internal-only` | Apply the selected policy only to transitions whose `map` value matches one of the selected input VMF file stems. External transitions are preserved and reported. |

This lets stitched campaigns clean up internal seams while keeping entry/exit transitions visible for later manual or external-tool validation.

## External preserve selectors

External transitions can be preserved by map, landmark, targetname, or a combination of those fields.

CLI job TOML:

```toml
changelevel_policy = "delete"
changelevel_scope = "all"
dry_run = true

[[preserve_external_transition]]
map = "external_entry"
landmark = "lm_exit"
targetname = "to_external"
```

Direct CLI merge flags:

```bash
sourceweaver merge \
  -o stitched_campaign.vmf \
  --changelevel-policy delete \
  --changelevel-scope internal-only \
  --preserve-external-map external_entry \
  changelevel_d1_a.vmf changelevel_d1_b.vmf
```

The direct flags create one preserve selector for each flag value. CLI jobs support richer combined selectors through repeated `[[preserve_external_transition]]` tables.

## Dry-run diff JSON

`sourceweaver run --job ...` reports the transition cleanup diff before writing output. With `dry_run = true`, `output_written` remains `false` and the diff is still included:

```json
{
  "dry_run": true,
  "output_written": false,
  "changelevel": {
    "policy": "delete",
    "scope": "all",
    "changed": [
      {
        "action": "delete",
        "old_map": "changelevel_d1_b",
        "landmark": "lm_exit",
        "rationale": "policy `delete` removes trigger_changelevel entities from the stitched output"
      }
    ],
    "preserved": [
      {
        "map": "external_entry",
        "landmark": "lm_exit",
        "targetname": "to_external",
        "reason": "external transition matched preserve rule ..."
      }
    ],
    "warnings": []
  }
}
```

The same report is available under `merge.changelevel` for merge jobs.

## Desktop UI

The desktop merge setup panel exposes:

- **Changelevel policy**: preserve, disable, delete, rewrite-internal;
- **Scope**: all or internal-only;
- **Preserve external by map / landmark / targetname** text fields.

Build-preview and export use the same core cleanup pass. Status messages show changed transition count, preserved transition count, warnings, and per-transition rationale. Saved desktop project files persist the policy, scope, and preserve fields so jobs can be replayed.

## Fixtures and tests

- `tests/jobs/transition-cleanup.toml` covers dry-run JSON with one deleted internal transition and one preserved external transition.
- `tests/fixtures/changelevel_d1_a.vmf` and `tests/fixtures/changelevel_d1_b.vmf` cover internal map-stem matching plus an external transition.
- Core tests cover `internal-only` cleanup and combined map/landmark/targetname preserve rules.
- CLI tests cover dry-run JSON and direct merge output/VMF behavior.
