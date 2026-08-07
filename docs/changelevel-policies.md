# Changelevel preservation and rewrite policies

Source Weaver can apply an explicit policy to top-level `trigger_changelevel` entities after a merge. These policies are portable VMF text edits. They do not run Hammer, Hammer++, VBSP, VVIS, VRAD, a game runtime, or any external SDK tool.

## Policies

| Policy | Behavior |
| --- | --- |
| `preserve` | Default. Leaves all `trigger_changelevel` entities unchanged. |
| `disable` | Keeps each `trigger_changelevel` entity and sets `StartDisabled` to `1` so destination metadata remains visible for review. |
| `delete` | Removes all top-level `trigger_changelevel` entities from the stitched output. |
| `rewrite-internal` | Rewrites destinations that target one of the selected input map stems to the output map stem. Destinations outside the selected map set are preserved. |

`rewrite-internal` is meant for many-map stitching where an internal transition such as `map "d1_b"` should point at the stitched output map. It uses selected VMF file stems as the internal map set, for example `d1_a.vmf` and `d1_b.vmf`. External entry/exit transitions remain unchanged because Source Weaver cannot infer whether they should become part of the stitched output.

## CLI merge usage

```bash
sourceweaver merge \
  -o stitched_campaign.vmf \
  --landmark lm_exit \
  --changelevel-policy rewrite-internal \
  d1_a.vmf d1_b.vmf
```

The merge command prints the chosen policy, the number of changed transition entities, policy warnings, and one line per changed transition with the rationale.

## CLI job usage

```toml
base = "../fixtures/changelevel_d1_a.vmf"
inputs = ["../fixtures/changelevel_d1_b.vmf"]
output = "../target/test-output/stitched_campaign.vmf"
landmark = "lm_exit"
changelevel_policy = "rewrite-internal"
dry_run = true

[delete]
protect_critical_entities = true
```

Job JSON reports include a top-level `changelevel` object and the same report under `merge.changelevel`:

```json
{
  "changelevel": {
    "policy": "rewrite-internal",
    "changed": [
      {
        "action": "rewrite-internal",
        "old_map": "changelevel_d1_b",
        "new_map": "stitched_campaign",
        "landmark": "lm_exit",
        "rationale": "policy `rewrite-internal` rewrites internal stitched-map destination `changelevel_d1_b` to output map `stitched_campaign` while leaving external destinations unchanged"
      }
    ],
    "warnings": []
  }
}
```

## Desktop usage

The desktop **Merge setup** panel includes a **Changelevel policy** selector. Preview and export use the same core policy pass. Status messages report the chosen policy, changed entity count, warnings, and per-transition rationale. Saved desktop project files include `changelevel_policy` so CLI jobs can replay the same choice.

## Landmark warnings

For policies that touch transition entities, Source Weaver checks whether a referenced `landmark` or `landmarkname` has a matching top-level `info_landmark` targetname in the merged VMF. Missing landmarks are reported as warnings in the policy report. The policy still applies because the report is a VMF-text edit, not a compile or runtime validator.

## Fixtures and coverage

- `tests/fixtures/changelevel_policy_internal.vmf` covers an internal transition.
- `tests/fixtures/changelevel_policy_external.vmf` covers one internal and one external transition.
- `tests/fixtures/changelevel_policy_missing_landmark.vmf` covers a missing landmark warning.
- `tests/fixtures/changelevel_d1_a.vmf` and `tests/fixtures/changelevel_d1_b.vmf` cover CLI job rewrite behavior where selected file stems form the internal map set.
- `tests/jobs/changelevel-rewrite.toml` covers CLI job JSON reporting.
