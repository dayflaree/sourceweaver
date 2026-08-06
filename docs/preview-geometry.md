# Preview geometry reconstruction

Source Weaver's 2D preview now uses reconstructed brush face polygons when possible instead of relying only on solid bounds.

## Reconstruction approach

For each VMF `solid`, the core preview extractor:

1. Reads every `side` `plane` triplet.
2. Computes an approximate solid center from all plane points.
3. Orients each plane so the solid center is inside the half-space.
4. Creates a large polygon on each face plane.
5. Clips that polygon against every other solid plane.
6. Keeps the resulting convex polygon when it has at least three vertices and non-zero area.

This produces real face polygons for closed convex Source brushes such as boxes, wedges, ramps, and many angled solids.

## Desktop rendering

The desktop preview projects reconstructed face polygons into the current top/front/side view and draws those polygons with the existing role/source/deletion/selection colors. Bounds rendering remains as a fallback for invalid, open, or otherwise unreconstructable solids. The original plane-point triangle lines remain visible as a low-cost debugging overlay.

## Test coverage

Core tests cover:

- axis-aligned boxes with six quadrilateral faces
- wedge/triangular-prism brushes with triangular and quadrilateral faces
- translation of reconstructed face polygons
- existing fixture coverage for mixed role and displacement maps

## Limitations

The current algorithm targets closed convex VMF brush solids. Non-convex geometry should already be represented by Hammer as multiple convex solids. Malformed or open solids fall back to bounds plus plane triangles instead of blocking preview or export.
