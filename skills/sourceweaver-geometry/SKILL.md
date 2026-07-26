---
name: sourceweaver-geometry
description: Implement robust Source brush geometry, transforms, seam overlap analysis, and generated-solid validation.
---

# Geometry kernel

## Trigger

Use for planes, solids, face polygons, texture axes, displacements, transforms, intersection, duplicate detection, portal/hint fitting, or spatial indexing.

## Procedure

1. Parse source numeric spelling and retain provenance.
2. Reconstruct each convex solid from half-space plane intersections.
3. Use accelerated floating-point predicates, then higher precision for ambiguous cases.
4. Verify boundedness, convexity, positive volume, face area, and edge length.
5. Keep approximate discovery geometry in separate types from commit-authority geometry.
6. Derive generated planes from exact source structural planes where possible.
7. Propagate transforms through every profile-typed geometry field.
8. Rebuild texture axes and editor precision only under qualified rules.
9. Validate world bounds and profile tolerances.
10. Compile golden fixtures and inspect resulting BSP topology.

## Required tests

- plane/side order permutations;
- axis-aligned and angled brushes;
- tiny/sliver/degenerate solids;
- coplanar and near-coplanar faces;
- touching, contained, exact duplicate, partial overlap, and conflicting solids;
- transform/inverse consistency;
- displacement rejection/handling;
- fuzz/property tests for bounded workloads.

## Blockers

- ambiguous predicate after precision fallback;
- non-convex/unbounded generated solid;
- unsupported displacement transform;
- feature below qualified minimum size;
- world-bound violation;
- compiler topology differs from expected.

## References

- `docs/GEOMETRY_KERNEL.md`
- `docs/GAME_PROFILES.md`
- `docs/TEST_STRATEGY.md`
