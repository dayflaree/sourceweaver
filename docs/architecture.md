# Source Weaver architecture

## Layers

### `sourceweaver-core`

The core crate owns VMF behavior:

- VMF tokenization and parsing
- VMF serialization
- entity and brush inspection
- preview geometry extraction for orthographic UI views
- VMF integrity validation for pre-write safety checks
- landmark discovery, duplicate checks, selected-landmark status, and origin lookup
- brush and entity translation
- merge operations
- deletion/prune operations

The core crate stays dependency-light and UI-agnostic so every interface shares the same map behavior.

### `sourceweaver-desktop`

The desktop app is a native egui/eframe application for Linux and Windows. It calls directly into `sourceweaver-core` and provides:

- VMF file selection through native file dialogs
- project/job TOML save/load for repeatable setups
- base-map selection
- discovered-landmark dropdown plus manual landmark targetname input
- per-map landmark status warnings before preview/export
- per-map VMF integrity status warnings before preview/export
- output VMF picker
- Hammer-style 2D orthographic VMF preview
- in-memory merged-output preview before export
- top, front, and side preview projections
- entity/classname inspection tables
- search, role filtering, filtered counts, and sorting for inspection tables
- entity table row-selection state for future cleanup actions
- deletion-rule controls
- transparent deletion presets that generate ordinary criteria
- deletion-safety controls for brush-entity modes and protected entities
- deletion preview
- cleaned-copy export
- merge/export action
- status log

The desktop app intentionally does not reimplement VMF logic. Any merge or deletion behavior change should be made in `sourceweaver-core` first.

### `sourceweaver-cli`

The CLI is retained for scripting, development validation, assistant-driven workflows, and regression tests.

Current commands:

- `inspect`
- `list-types`
- `prune`
- `merge`
- `run` / `batch` / `job` for non-interactive TOML job execution
- `job-template` for generating a starter automation file

The job runner resolves relative paths from the job file directory, can dry-run the operation, and emits a JSON report for machine parsing. The desktop app writes and reads the same TOML shape where possible, with relative paths anchored to the project file directory.

## VMF model

Source Weaver currently represents VMF as a generic ordered KeyValues tree:

```text
Document
  Node::Block { name, body }
  Node::Property { key, value }
```

This preserves unknown Hammer and game-specific data because the parser does not discard unrecognized blocks or keys.


## Preview model

`sourceweaver-core` extracts preview data from the same parsed VMF tree used by merge and deletion. The first preview slice collects brush side `plane` points, computes solid bounds, classifies each solid with the existing brush-role logic, and records entity `origin` markers.

The desktop app renders this data in three orthographic projections:

- top: X/Y
- front: X/Z
- side: Y/Z

This gives a Hammer-style 2D map overview for verifying rough layout and landmark alignment. The desktop app can render either the selected source VMF or an in-memory merged result generated from the current base map, landmark, and deletion rules. It is not yet a full textured 3D renderer.

## Merge model

The first selected VMF is the base document. For each additional VMF:

1. Parse VMF.
2. Optionally prune content using the selected deletion criteria.
3. Discover selected-map `info_landmark` targetnames for UI status and warnings.
4. Find requested `info_landmark` targetname.
5. Compute translation offset against the base landmark.
6. Translate incoming entity `origin` values.
7. Translate incoming brush `plane` values.
8. Translate displacement `startposition` values when present.
9. Renumber incoming `id` keys.
10. Append incoming world solids into the base `world` block.
11. Append incoming top-level entities after existing base nodes.

Landmark discovery records top-level `entity` blocks with `classname` `info_landmark` and a non-empty `targetname`. A landmark needs a parseable `origin` before it can drive alignment. The desktop UI still shows duplicate and invalid-origin status so users can fix ambiguous maps before merging.

Integrity validation runs before preview/export write paths. The core validator reports structural errors such as missing or duplicate top-level `world` blocks and warnings such as missing common VMF sections, missing IDs, duplicate numeric IDs, multiple ID fields, or non-numeric IDs on blocks where Hammer normally expects stable IDs.

## Deletion model

Deletion is criteria-based. The UI builds a `DeletionCriteria` object from selected filters, then calls the same prune function as the CLI.

Brush-role deletion removes matching world solids directly. Brush entities use an explicit safety mode: `whole-entity` preserves the historical behavior where matching brush entities are removed as a unit, while `matching-solids` keeps brush entities and removes only matching contained solids. Critical transition/player/logic classnames are protected by default unless the CLI job or desktop UI explicitly disables protection.

Deletion presets live in the desktop layer and produce ordinary `DeletionCriteria`. The preview button runs the same pruning path as final cleanup/merge export, so preset preview counts are generated by the core deletion engine rather than by a separate estimator.

## Cross-platform strategy

Rust and egui/eframe provide a shared Linux/Windows desktop UI. Native file dialogs are handled through `rfd`. CI builds the Rust workspace on Linux and Windows so platform-specific compile issues surface quickly.

## Known technical risks

- Displacement data may need more coordinate translation beyond `startposition`.
- Texture lock behavior may require updating texture-axis offsets after brush translation.
- Some maps contain nested/hidden groups that need more nuanced merge behavior.
- VMF instance handling may require expanding or preserving `func_instance` workflows.
- Hammer compile limits may be reached when many campaign maps are merged.
