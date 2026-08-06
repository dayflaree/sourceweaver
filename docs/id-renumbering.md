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

Unknown or game-specific keys are preserved as-authored until a fixture proves they reference VMF IDs.

## Duplicate IDs in input maps

Some VMFs contain duplicate old IDs. In that case, an old ID can map to multiple new IDs after renumbering. Source Weaver does not guess which new ID a reference should target. Ambiguous references are intentionally left unchanged so they do not silently point at the wrong new object.

The integrity checker still warns about duplicate numeric IDs so users can inspect the source VMF.

## Test coverage

Regression tests cover:

- incoming ID renumbering across world solids, sides, brush entities, and point entities
- `sides` list remapping
- single-ID reference remapping
- leaving unknown IDs unchanged
- leaving ambiguous duplicate-ID references unchanged

## Adding new reference fields

Before adding a new remapped key, add a VMF fixture that demonstrates the field references a VMF `id` value and a test that verifies the expected old-to-new rewrite.
