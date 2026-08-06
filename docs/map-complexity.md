# VMF complexity risk heuristics

Source Weaver computes a portable VMF complexity summary during validation. The report is a heuristic preflight check for large stitched maps. It does not run Hammer, VBSP, VVIS, VRAD, a game runtime, or any external SDK tool, and it does not prove that a map will compile, fail to compile, load, or fail to load.

## Reported counts

The `complexity` report includes counts that can be computed from VMF text:

- top-level entities;
- point entities;
- brush entities;
- brush solids, counting `solid` blocks under world and brush entities;
- sides/faces, counting VMF `side` blocks;
- displacements, counting `dispinfo` blocks;
- overlays, counting `info_overlay` entities.

## Thresholds

Source Weaver currently uses public Source SDK BSP constants as heuristic caps and warns at 75% of each cap. Branches and games vary, and VMF-side counts do not map perfectly to compiled BSP lumps. The warnings are intentionally advisory.

| Metric | Heuristic limit | Warns at |
| --- | ---: | ---: |
| entities | 4,096 | 3,072 |
| brush solids | 8,192 | 6,144 |
| brush sides | 65,536 | 49,152 |
| faces | 65,536 | 49,152 |
| displacements | 2,048 | 1,536 |
| overlays | 512 | 384 |

Threshold source: public Valve SDK `bspfile.h` constants observed from the Source SDK mirror used during implementation. Source Weaver records these as heuristic constants in `crates/sourceweaver-core/src/complexity.rs`.

## CLI JSON

```bash
sourceweaver validate map.vmf --json
```

Relevant fields:

```json
{
  "complexity": {
    "entities": 3,
    "point_entities": 2,
    "brush_entities": 1,
    "brush_solids": 2,
    "sides": 2,
    "displacements": 1,
    "overlays": 1,
    "warnings": 0,
    "risks": []
  }
}
```

Each entry in `risks` is a warning with `metric`, `count`, `warn_at`, `limit`, and a message that repeats the VMF-only uncertainty boundary.

## Desktop UI

The desktop VMF integrity panel shows complexity warning counts alongside integrity, entity-semantic, and rule-set counts. Detailed risk messages appear under **Complexity risk details** when a loaded map reaches a threshold.

## Fixture coverage

- `tests/fixtures/complexity_counts.vmf` verifies the CLI/core count shape.
- Unit tests synthesize near-limit overlay counts to verify risk classification without creating a huge checked-in VMF.
