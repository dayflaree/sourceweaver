# Geometry kernel

## Requirements

Committed brush transformations must remain valid under Source compiler rules. The kernel must support:

- convex solids defined by half-spaces;
- finite face polygon reconstruction;
- exact/robust sidedness tests;
- coplanar face matching;
- world and brush-entity transforms;
- texture axis transformation;
- displacement restrictions;
- overlap and duplicate classification;
- portal/hint brush fitting;
- numerical diagnostics.

## Representations

### Exact-authority representation

Use normalized plane equations and robust predicates. Coordinates may begin as decimal/rational representations derived from VMF strings. Floating-point acceleration is allowed, but ambiguous predicates fall back to higher precision.

### Approximate discovery representation

Use adaptive voxels/octrees, navigation samples, or signed-distance fields for:

- room/corridor segmentation;
- exterior flood;
- choke-point discovery;
- candidate visibility samples;
- rough overlap localization.

Approximate data cannot directly emit a brush. It must fit against exact source planes and pass exact validation.

## Brush reconstruction

For each solid:

1. Parse all side planes.
2. Normalize orientation consistently.
3. Intersect every non-parallel plane triple.
4. Keep points inside all half-spaces within a branch-qualified tolerance.
5. Deduplicate vertices with robust clustering.
6. Build each face polygon by selecting vertices on its plane and sorting in plane coordinates.
7. Verify positive volume, boundedness, convexity, and at least four valid planes.
8. Compare reconstructed face topology with `vertices_plus` when present.

A failure marks the brush non-transformable; analysis may continue.

## Tolerances

No universal epsilon is hardcoded as truth. Profiles define:

- plane distance tolerance;
- vertex merge tolerance;
- coplanarity angular/distance tolerance;
- minimum edge length;
- minimum face area;
- minimum brush volume;
- grid/world-bound margins.

Qualification maps determine safe defaults. Reports include every tolerance used.

## Transform propagation

A translation affects:

- side plane points;
- Hammer++ precise vertices;
- entity origins and typed coordinate fields;
- displacement start positions;
- overlays and face references where geometry IDs change;
- cubemap sample positions;
- ropes and path nodes;
- cameras and cordons;
- editor group bounds where represented.

A rotation additionally affects:

- plane normals;
- angles and local basis fields;
- texture U/V axes;
- displacement orientation/neighbors;
- overlay basis vectors;
- model orientation and branch-specific fields.

Initial stitching supports translation-only alignment to reduce this surface.

## Duplicate geometry classes

- **Byte-equivalent after transform:** same semantic planes/materials/side data.
- **Plane-equivalent:** same convex volume, different IDs/order/formatting.
- **Visually equivalent:** similar visible surfaces, different hidden structure.
- **Overlapping compatible:** partial overlap that could be retained without invalid intersection.
- **Conflicting:** overlapping solids with incompatible content/material/entity semantics.

Only byte-equivalent and qualified plane-equivalent duplicates may be removed automatically in the first release.

## Generated-solid acceptance

A generated solid must:

- be convex and bounded;
- have finite coordinates;
- meet minimum feature sizes;
- remain inside profile world bounds;
- have unique IDs;
- use valid tool materials;
- compile without new warnings;
- produce expected BSP topology.
