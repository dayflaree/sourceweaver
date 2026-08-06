# Entity semantic validation

Source Weaver validates entity targetname semantics as a portable VMF-text pass. This pass runs without Hammer, VBSP, VVIS, VRAD, a game runtime, game content, SDKs, or custom assets.

The CLI report exposes findings under `entity_semantics`, separate from generic VMF `integrity`, optional `rule_set`, and optional compile-log fields.

## Duplicate targetnames

Source I/O can intentionally address multiple entities with the same `targetname`, so Source Weaver does not treat every duplicate as a hard error.

Current behavior:

- duplicate targetnames on likely group-targeted entities are warnings;
- duplicate targetnames involving likely unique/addressable classes are higher-risk warnings;
- the first likely-unique class list is intentionally conservative: `info_landmark`, `path_track`, `path_corner`, `phys_constraint`, and `logic_case`.

Keeping duplicate targetnames as warnings avoids blocking maps that intentionally use target groups or merged transition seams while still surfacing potential merge hazards for human review.

## Missing target references

Source Weaver checks common target-reference properties against targetnames present in the same VMF:

- `target`
- `parentname`
- `filtername`
- `landmark`
- `landmarkname`
- Source I/O output keys starting with `On`, using the first comma-separated value as the target entity name

The pass skips intentionally dynamic or special references that would otherwise create false positives, including `!self`, `!activator`, `!caller`, `!player`, wildcard patterns containing `*` or `?`, `player`, and `worldspawn`.

## Fixtures

- `tests/fixtures/entity_semantics_issues.vmf` covers an unsafe duplicate `info_landmark` targetname plus missing output/filter references.
- `tests/fixtures/entity_semantics_group_warning.vmf` covers intentional duplicate group targetnames that remain warnings and keep validation successful.

## Example

```bash
sourceweaver validate tests/fixtures/entity_semantics_issues.vmf --json
```

Relevant JSON fields:

```json
{
  "entity_semantics": {
    "errors": 0,
    "warnings": 3,
    "issues": [
      {
        "category": "duplicate-targetname",
        "rule_id": "entity.duplicate_targetname",
        "targetname": "exit_a"
      },
      {
        "category": "missing-target-reference",
        "rule_id": "entity.missing_target_reference",
        "key": "OnTrigger",
        "targetname": "door_missing"
      }
    ]
  }
}
```

A passing semantic report only means Source Weaver's portable entity checks passed. Real compile or runtime validation still requires running the relevant external tools and recording that evidence.
