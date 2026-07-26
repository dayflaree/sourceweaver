# Visibility optimization

## Principle

A visibility change is accepted through measured compiler/runtime improvement. Geometric appearance alone never proves performance benefit.

## Baseline

Compile the untouched VMF with the exact profile and collect:

- VBSP/VVIS/VRAD duration and peak memory where observable;
- BSP planes, nodes, leaves, faces, clusters, portals, areas, areaportals;
- PVS lump size and per-cluster visibility statistics;
- runtime visible leaves/clusters, world faces, props, draw calls, triangles, and frame time at deterministic samples;
- compiler warnings and leak files.

## Nodraw candidates

Candidate evidence may include:

- a face is fully covered by opaque, permanent world geometry;
- a face borders sealed unreachable solid/void space;
- all supported player/camera samples fail to see it;
- the face is internal to plane-equivalent touching solids.

Block or require review for:

- breakables or removable occluders;
- moving brush entities;
- mirrors, render targets, portals, cameras, or scripted views;
- skybox visibility;
- translucent/alphatest materials;
- displacements;
- uncertain custom entities.

Material replacement is accepted only when compile/runtime image and behavior checks pass.

## `func_detail` candidates

A world brush can be proposed when it:

- does not seal reachable space from the exterior;
- does not form an areaportal boundary;
- does not need to split visibility for a measured occluder;
- does not participate in water, displacement, special contents, or profile-specific behavior;
- contributes disproportionate BSP cuts.

The candidate is compiled in isolation and accepted when leak status remains clean and objective metrics improve within leaf/PVS budgets.

## Areaportal detection

### Candidate discovery

1. Build an adaptive empty-space graph from reachable space.
2. Identify narrow connections between larger cell components.
3. Test whether removing a connection splits the graph.
4. Fit an opening polygon against exact sealing world planes.
5. Detect overlapping door geometry and supported door semantics.
6. Reject exterior connections, bypass paths, multi-opening cuts, and unsupported dynamic boundaries.

### Exact fitting

The fitted portal brush must:

- be one convex brush;
- use the profile's areaportal material;
- touch sealing geometry around its boundary;
- avoid sliver faces and ambiguous intersections;
- align with an existing structural plane where possible;
- remain independent from the moving door brush.

### Compiler proof

The candidate must:

- pass VBSP leak testing;
- touch exactly two compiled areas;
- produce no `> 2 areas` warning;
- create the expected area relationship in BSP data;
- preserve door behavior;
- improve the configured visibility objective enough to offset runtime portal overhead.

A closed areaportal is linked to a door only through a profile-qualified rule.

## Hint/skip generation

### Candidate locations

- L/T/S corridor turns;
- leaf boundaries extending around a structural occluder;
- doorway/header planes;
- tall wall termination under open vertical space;
- large leaves crossing room boundaries;
- existing world planes that can form clean axial cuts.

### Candidate constraints

- derive from source structural planes;
- one intended hint face; remaining faces use skip;
- fit to sealing boundaries;
- avoid excessive plane/leaf generation;
- never blanket-fill the map with hints.

### Search

Use a bounded experiment search:

1. rank candidates by predicted visibility benefit;
2. compile one candidate at a time against the same baseline;
3. reject invalid/regressive candidates;
4. test compatible combinations with an explicit budget;
5. run full VVIS for finalists;
6. run runtime samples;
7. select the Pareto-optimal accepted set.

The search is deterministic for a given seed and candidate list.

## Occluders and visclusters

`func_occluder` candidates may be explored only through measured runtime benefit because occluder testing has CPU cost and affects models rather than world PVS in the same way.

The current GMod Tools++ profile does not generate `func_viscluster`. VBSP++ documents that entity as deprecated when using VVIS++.

## Objective example

```text
score =
  0.40 * normalized_p95_frame_time_improvement
+ 0.25 * normalized_p95_visible_cluster_reduction
+ 0.20 * normalized_visible_prop_reduction
+ 0.15 * normalized_bsp_size_improvement
- penalties(leaf_growth, portal_overhead, compile_time, warnings)
```

Weights are profile/policy inputs and are always reported.
