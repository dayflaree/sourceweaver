# VMF ID renumbering and reference remapping

Source Weaver renumbers incoming VMF IDs during merge so appended world solids, sides, entities, and nested objects do not collide with IDs already present in the base map.

## Remapped reference fields

During each incoming map merge, Source Weaver builds an old-to-new ID map, then rewrites known ID reference properties when the old ID maps to exactly one new ID.

Single-ID reference keys currently remapped:

- `parentid`
- `groupid`
- `visgroupid`
- `sideid`
- `solidid`
- `entityid`
- `nodeid`

List reference keys currently remapped:

- `sides`, as used by overlay-style entities to reference side IDs

These keys live in `crates/sourceweaver-core/src/id_references.rs`, which is shared by merge remapping and integrity warnings so the supported field list stays consistent.

Unknown or game-specific keys are preserved as-authored until a fixture proves they reference VMF IDs.

## Unknown suspected ID-reference warnings

The integrity checker warns when it sees a numeric property whose key looks like an ID-reference field but is not in the supported remap list. The heuristic currently flags unsupported keys ending in `id` or `ids` when the value is a numeric ID or whitespace-separated numeric ID list.

Known ID-like non-reference keys are ignored:

- `id`, which is the owned object ID that Source Weaver already renumbers;
- `hammerid` and `hammeruniqueid`, which can appear as editor/tool metadata but are not treated as local VMF reference fields;
- `visgroupshown` and `visgroupautoshown`, which appeared in real HL2 VMFs as boolean view/editor fields rather than ID references;
- view/editor grid booleans ending in `Grid`, such as `bSnapToGrid` and `bShow3DGrid`.

Example warning:

```text
entity[0] property `targetid` has numeric value `1` and looks like an unsupported VMF ID-reference field; Source Weaver currently remaps single: parentid, groupid, visgroupid, sideid, solidid, entityid, nodeid; list: sides; add a fixture before enabling automatic remap
```

## Real-VMF inventory on 2026-08-06

The public validation corpus used by `scripts/validate-public-vmfs.sh` was scanned after downloading the HL2 chapter VMFs from the `rubycho/labescape-hl2` fixture source at commit `184f8c5eec17313724155f91f2f99133c12c464a`.

Inventory result:

- verified ID-reference fields already covered by Source Weaver: nested editor `groupid` values;
- no additional unsupported `*id` or `*ids` field was verified as a VMF ID reference in that corpus;
- numeric non-reference fields observed in the same corpus included `visgroupshown`, `visgroupautoshown`, `spawnflags`, `fadescale`, light scalars, colors, and coordinates.

The inventory keeps the supported remap list conservative and adds warnings for unknown ID-like fields instead of silently ignoring them.

## Duplicate IDs in input maps

Some VMFs contain duplicate old IDs. In that case, an old ID can map to multiple new IDs after renumbering. Source Weaver does not guess which new ID a reference should target. Ambiguous references are intentionally left unchanged so they do not silently point at the wrong new object.

The integrity checker still warns about duplicate numeric IDs so users can inspect the source VMF.

## Test coverage

Regression tests cover:

- incoming ID renumbering across world solids, sides, brush entities, and point entities;
- fixture-backed remapping for every supported single-ID key and the `sides` list key through `tests/fixtures/id_reference_remap_fields.vmf`;
- warning on unsupported suspected ID-reference keys through `tests/fixtures/id_reference_suspected_unknown.vmf`;
- leaving unknown IDs unchanged;
- leaving ambiguous duplicate-ID references unchanged.

## Adding new reference fields

Before adding a new remapped key, add a VMF fixture that demonstrates the field references a VMF `id` value and a test that verifies the expected old-to-new rewrite.
