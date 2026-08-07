# Campaign adjacency graph

Source Weaver builds a campaign adjacency graph from selected VMFs during CLI job reporting. The graph keeps explicit `trigger_changelevel` evidence separate from heuristic evidence and assigns a confidence label to every edge.

This is a VMF-text and selected-file-list analysis only. It does not run Hammer, VBSP, VVIS, VRAD, a game runtime, or any external SDK tool.

## Research summary

The reliable Source map-adjacency signal remains `trigger_changelevel` with a target `map` and optional landmark. Non-trigger signals are weaker because Source VMFs do not have a universal campaign manifest field. External files such as map lists, chapter manifests, compile logs, or mod-specific scripts can describe order in some projects, but Source Weaver does not currently parse those files. The first implementation therefore limits heuristics to evidence already available in the selected VMF set:

- selected VMF file stems with sequential numeric suffixes;
- unique shared `info_landmark` targetnames across exactly two maps.

Heuristic edges are advisory and never overwrite or merge with explicit trigger edges.

## Evidence kinds

| Evidence kind | Confidence | Meaning |
| --- | --- | --- |
| `trigger_changelevel` | `high` | A VMF `trigger_changelevel` target map matched another selected VMF. |
| `shared_landmark` | `medium` | Exactly two selected maps contain the same `info_landmark` targetname and there is no explicit edge between them. Direction is heuristic. |
| `filename_sequence` | `low` | Selected VMF file stems share a prefix and have adjacent numeric suffixes, such as `d1_trainstation_01` → `d1_trainstation_02`; direction follows sorted filename order. |

## JSON report

`sourceweaver run --job ...` includes `campaign_adjacency`:

```json
{
  "campaign_adjacency": {
    "edges": [
      {
        "from_map": "campaign_adjacency_01.vmf",
        "to_map": "campaign_adjacency_02.vmf",
        "evidence_kind": "trigger_changelevel",
        "confidence": "high",
        "evidence": "trigger_changelevel #1 targets map `campaign_adjacency_02` with landmark Some(\"adj_lm\")"
      },
      {
        "from_map": "campaign_adjacency_02.vmf",
        "to_map": "campaign_adjacency_03.vmf",
        "evidence_kind": "filename_sequence",
        "confidence": "low",
        "evidence": "map file stems share prefix `campaign_adjacency_` and sequential numbers 2 -> 3; direction follows sorted filename order"
      }
    ],
    "warnings": []
  }
}
```

Explicit transition warnings stay in the graph warnings list when a trigger has no target map, points at a missing selected map, or points at itself.

## Fixtures and tests

- `tests/fixtures/campaign_adjacency_01.vmf` contains an explicit transition to `campaign_adjacency_02`.
- `tests/fixtures/campaign_adjacency_02.vmf` and `tests/fixtures/campaign_adjacency_03.vmf` exercise filename-sequence inference beyond explicit transitions.
- `tests/jobs/campaign-adjacency.toml` verifies CLI job JSON contains both explicit and heuristic edges.
- Core campaign tests verify explicit edges suppress duplicate heuristic edges and that shared-landmark inference stays separate from explicit trigger evidence.
