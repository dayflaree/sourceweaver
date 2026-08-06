# Source Weaver architecture

## Layers

### `sourceweaver-core`

The core crate owns VMF behavior:

- VMF tokenization and parsing
- VMF serialization
- entity and brush inspection
- campaign transition discovery for `trigger_changelevel` entities
- preview geometry extraction for orthographic UI views
- VMF integrity validation for pre-write safety checks
- Source-tool validation reports and VBSP compile-log parsing
- optional VBSP/VVIS/VRAD compile pipeline orchestration
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
- source-colored merged-output preview metadata and legend
- landmark markers, selected-landmark labels, and merge-offset arrow overlays
- top, front, and side preview projections
- entity/classname inspection tables
- transition table grouping detected `trigger_changelevel` target maps and landmarks
- transition-derived campaign order and landmark-pair suggestions
- search, role filtering, filtered counts, and sorting for inspection tables
- built-in, inferred, and FGD-loaded entity metadata in inspection tables
- drag-and-drop VMF/project/FGD import and in-session recent paths
- parse progress, error dialog, theme toggle, and adjustable preview height controls
- entity table row-selection state for future cleanup actions
- deletion-rule controls
- transparent deletion presets that generate ordinary criteria
- deletion-safety controls for brush-entity modes and protected entities
- deletion preview
- visual deletion overlay modes in the selected-map preview
- preview click selection synchronized with entity table rows
- pending cleanup review, undo, and confirmation state
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
- `validate` for portable Source-tool readiness checks and optional VBSP log parsing/execution
- `compile` for optional user-configured VBSP/VVIS/VRAD execution with log capture
- `run` / `batch` / `job` for non-interactive TOML job execution
- `job-template` for generating a starter automation file

The job runner resolves relative paths from the job file directory, can dry-run the operation, and emits a JSON report for machine parsing. The desktop app writes and reads the same TOML shape where possible, with relative paths anchored to the project file directory.

Desktop usability is intentionally synchronous and native. Loading VMFs, projects, and FGD files can be started with file dialogs, drag-and-drop, or recent-path shortcuts. The selected-map list shows parse progress and failures, and important failures also surface in a dismissible error dialog so users do not need terminal logs.

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

Merged-preview source coloring is preview-only metadata. It does not write synthetic source keys into exported VMFs. The desktop preview builds a source-tagged preview from the same pruned input documents and the merge report's computed offsets, so selected map order gives stable source colors for the current selection.

For closed convex brushes, the preview extractor reconstructs real face polygons by clipping each side plane against the other side planes. Bounds rendering remains as a fallback for malformed/open solids. See `docs/preview-geometry.md`.

The preview UI also offers a 3D isometric camera mode with yaw/pitch controls. It reuses the same reconstructed polygons, entity markers, landmark markers, source colors, deletion overlays, and selection outlines as the 2D views. See `docs/3d-preview.md`.

Landmark markers are extracted as explicit preview metadata from `info_landmark` entities. The selected merge landmark is drawn with a stronger marker/label. Merged previews also render per-source offset arrows using the same offset values reported by `merge_maps`.

## Merge model

The first selected VMF is the base document. For each additional VMF:

1. Parse VMF.
2. Optionally prune content using the selected deletion criteria.
3. Discover selected-map `info_landmark` targetnames for UI status and warnings.
4. Find requested `info_landmark` targetname.
5. Compute translation offset against the base landmark.
6. Translate incoming entity `origin` values.
7. Translate incoming brush `plane` values.
8. Translate displacement `startposition` values when present, including square-bracket VMF displacement syntax.
9. Adjust `uaxis`/`vaxis` texture offsets to keep textures locked to translated brushes.
10. Renumber incoming `id` keys and remap known ID reference fields.
11. Append incoming world solids into the base `world` block.
12. Append incoming top-level entities after existing base nodes.

Base top-level editor metadata remains in place. Incoming top-level editor metadata is intentionally ignored, while nested metadata inside appended objects is preserved. See `docs/editor-metadata.md`.

Landmark discovery records top-level `entity` blocks with `classname` `info_landmark` and a non-empty `targetname`. A landmark needs a parseable `origin` before it can drive alignment. The desktop UI still shows duplicate and invalid-origin status so users can fix ambiguous maps before merging.

Integrity validation runs before preview/export write paths. The core validator reports structural errors such as missing or duplicate top-level `world` blocks and warnings such as missing common VMF sections, missing IDs, duplicate numeric IDs, multiple ID fields, or non-numeric IDs on blocks where Hammer normally expects stable IDs.

Campaign transition discovery records top-level `trigger_changelevel` entities, including targetname, target map, landmark/landmarkname, origin, and trigger solid count. CLI inspection, automation reports, and the desktop transition table surface this data so later landmark-ordering workflows can use it.

Campaign suggestion builds a lightweight graph from selected VMF labels and `trigger_changelevel` target map values. It suggests a topological order, emits landmark-pair candidates, and warns about missing target maps or target maps that lack the referenced landmark. Suggestions remain advisory; desktop users can apply the suggested order/first landmark or keep manual ordering and landmark entry.

## Source tool validation

The CLI `validate` command loads a VMF, runs integrity checks, optionally executes a configured VBSP command, and parses captured compiler logs. CI uses the portable mode plus fixture logs because hosted Linux/Windows runners do not include Hammer or game-specific Source tool installations.

The CLI `compile` command extends this into an optional Source compile pipeline. Users can provide tool paths directly or through a TOML profile, select VBSP/VVIS/VRAD steps, capture stdout/stderr logs, and emit a JSON report with parsed warnings, errors, leaks, and exit codes. See `docs/compile-pipeline.md`.

## ID renumbering

Each incoming map gets an old-to-new ID map during merge. Known reference fields such as `parentid`, `groupid`, `visgroupid`, `sideid`, `solidid`, `entityid`, `nodeid`, and overlay `sides` are rewritten when the old ID has exactly one new target. Ambiguous duplicate old IDs are intentionally left unchanged instead of guessed. See `docs/id-renumbering.md`.

## Editor metadata

The base VMF supplies top-level editor metadata such as `versioninfo`, `viewsettings`, `visgroups`, `cameras`, and `cordons`. Incoming top-level metadata is ignored to avoid conflicting global editor state. Nested editor metadata inside appended solids/entities is preserved with those objects.

## Entity metadata

The core library provides class-level metadata for common Source entities, infers broad categories from unknown classname prefixes, and can parse lightweight FGD class declarations for class descriptions. Desktop inspection tables display category, friendly name, and description without hiding unknown classnames. See `docs/entity-metadata.md`.

## Deletion model

Deletion is criteria-based. The UI builds a `DeletionCriteria` object from selected filters, then calls the same prune function as the CLI.

Brush-role deletion removes matching world solids directly. Brush entities use an explicit safety mode: `whole-entity` preserves the historical behavior where matching brush entities are removed as a unit, while `matching-solids` keeps brush entities and removes only matching contained solids. Critical transition/player/logic classnames are protected by default unless the CLI job or desktop UI explicitly disables protection.

Deletion presets live in the desktop layer and produce ordinary `DeletionCriteria`. The preview button runs the same pruning path as final cleanup/merge export, so preset preview counts are generated by the core deletion engine rather than by a separate estimator.

The selected-map preview also computes a non-destructive visual overlay from the current deletion criteria. Users can highlight, dim, hide, or disable matched preview solids/entity markers. Merged preview remains the already-pruned in-memory result and reports exact removal counts from the core prune path.

Preview click selection is scoped to the selected VMF. Entity markers and solid bounds resolve to the owning world/entity record, toggle the same selection key used by the entity table, and switch the inspection table to the entity view. Selection outlines remain visible across top/front/side projections because selection keys are stored independently of the active projection.

Cleanup exports are gated by a pending-review state. Running deletion preview stores the criteria and exact prune counts. If criteria change, the review is stale and confirmation is revoked. Cleaned-copy and merge export only write destructive cleanup when the current criteria match the pending review and the user has clicked **Confirm cleanup export**. Undo clears the pending review and confirmation.

## Cross-platform strategy

Rust and egui/eframe provide a shared Linux/Windows desktop UI. Native file dialogs are handled through `rfd`. CI builds the Rust workspace on Linux and Windows so platform-specific compile issues surface quickly.

Desktop release builds run from `.github/workflows/desktop-builds.yml`. Manual dispatch creates workflow artifacts. Pushing a `v*` tag builds the Linux tarball and Windows zip, then publishes both to a GitHub Release using `CHANGELOG.md` as release notes.

## Regression fixtures

Regression tests include representative VMF fixtures and golden snapshots. Core integration tests cover role classification, duplicate landmarks, transition discovery, preview extraction, prune counts, malformed input errors, and merged VMF golden output. CLI integration tests snapshot the job-runner JSON report and verify malformed inputs produce actionable filename/byte-position errors.

## Known technical risks

- Real displacement-heavy maps may reveal game-specific edge cases; current behavior is documented in `docs/displacements.md`.
- Real textured map captures may reveal material-specific edge cases; current texture-axis behavior is documented in `docs/texture-axes.md`.
- Some maps contain nested/hidden groups that need more nuanced merge behavior.
- VMF instance handling may require expanding or preserving `func_instance` workflows.
- Hammer compile limits may be reached when many campaign maps are merged.
