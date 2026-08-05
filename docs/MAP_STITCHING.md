# Map stitching

## Initial support envelope

The first production stitcher targets **two directly connected original VMFs** for the same game profile when:

- each transition edge has one unambiguous `trigger_changelevel`;
- both maps contain the named `info_landmark`;
- the inferred transform is translation-only;
- all transformed entity fields have known FGD/schema types;
- transition overlap can be classified without destructive boolean operations;
- scripts and custom entities do not contain unresolved dependencies on the transition;
- the combined map fits the profiled world and BSP budgets;
- compiler and runtime gates pass.

Inputs outside this envelope remain analyzable and receive blocker diagnostics.

## Implemented support envelope

The current repository implementation provides a read-only transition graph slice:

- extracts `trigger_changelevel` entities and their direct `map` and `landmark` keyvalues;
- normalizes destination map names case-insensitively and strips a trailing `.bsp` only for matching;
- extracts `info_landmark` entities by direct `targetname` and parses direct `origin` vectors;
- extracts `trigger_transition` entities and indexes direct `targetname` and `landmark` values as possible transition-volume names;
- links changelevel edges to uniquely matching landmarks and same-named transition volumes;
- reports deterministic blockers for missing map keys, missing landmark keys, missing landmarks, duplicate landmarks, and invalid landmark origins;
- surfaces transition blockers through `sourceweaver inspect` as `STITCH001` diagnostics.
- builds a read-only translation hypothesis between two transition graphs when there is exactly one A→B edge, exactly one B→A edge, matching landmark names, and validated landmark origins on both sides;
- computes the candidate-to-source offset as `source_landmark_origin - candidate_landmark_origin` and blocks nonfinite results.
- builds read-only seam overlap evidence by translating candidate brush records in memory, selecting AABB candidate pairs, and attaching exact convex brush relation classifications.
- classifies seam brush pairs into review-only deletion evidence: equal-volume candidate duplicates, candidate-contained-in-source, source-contained-in-candidate, touching seams to preserve, unsafe overlaps to preserve, and unclassified preserve cases.
- summarizes bounded seam confidence as review-ready only when deletion evidence is valid, seam evidence is non-empty, unsafe overlaps are absent, and source-map brush removal is not required.
- plans imported candidate IDs for entity, solid, and side objects by allocating fresh positive decimal IDs above all observed source and candidate IDs; duplicate, missing, and non-numeric candidate IDs block the plan.
- plans CST-backed targetname namespacing for direct candidate `targetname` definitions plus resolved `parentname` and output-target references; empty prefixes, source collisions, unresolved references, ambiguous references, special references, and wildcard references block the plan.
- reports baseline world/singleton conflict evidence for selected worldspawn keys and known singleton controller classes, including candidate duplicate singletons.
- aggregates alignment, seam confidence, ID allocation, namespace planning, singleton conflict evidence, and candidate import capacity limits into a read-only stitch preflight report.
- assembles a read-only stitch plan manifest from ready preflight evidence, including candidate-to-source offset, candidate removal keys, ID allocations, and namespace edits.
- includes a synthetic `transition_alpha`/`transition_beta` fixture pair that exercises transition extraction, alignment, seam duplicate evidence, ID allocation, namespacing, preflight, and manifest assembly.
- materializes a reversible generated VMF for the current synthetic envelope by preserving the source VMF as an exact output prefix and appending rewritten candidate `entity` blocks with source-to-output provenance.
- authorizes candidate duplicate removals only when the stitch manifest is valid, seam confidence is review-ready, every removal has material-equivalence evidence, the compiler preflight is ready, runtime acceptance has passed, and every removal is an exact equal-volume or contained-candidate seam class.
- extends targetname namespacing to a conservative built-in FGD keyvalue schema for selected Source classes, while leaving unknown string fields opaque.
- builds a deterministic read-only lifecycle controller plan by expanding clear lifecycle policies into ordered preload, activate, deactivate, reset, and remove steps for one named region.
- builds read-only lifecycle controller entity specs by assigning each lifecycle phase to a generated `logic_relay` specification.

This slice never mutates source inputs. Generated output is a new VMF byte stream and source bytes remain preserved as an exact prefix. Alignment, seam overlap, deletion-class, seam-confidence, ID-allocation, namespace-plan, singleton-conflict, preflight, manifest, materialization, and lifecycle-controller records do not authorize overwriting originals. The implementation does not materialize brush geometry transforms, remove duplicate transition geometry from source files, append lifecycle controller entities into generated VMFs, wire controller outputs, or run compiler/runtime acceptance gates. FGD-backed namespacing is intentionally limited to the documented built-in conservative keyvalue schema until exact game FGDs are wired in.

## Pipeline

### 1. Fingerprint and parse

- Hash every input VMF and referenced profile.
- Verify byte-identical lossless round trips.
- Build semantic, entity-I/O, geometry, instance, and resource indexes.
- Reject malformed solids only from transformation eligibility; continue analysis where safe.

### 2. Build the transition graph

For every `trigger_changelevel`, record:

- source map;
- destination map string;
- landmark name;
- trigger solid bounds;
- matching `info_landmark` origin;
- all same-named `trigger_transition` solids;
- transition-adjacent entities and geometry;
- map-local scripts and lifecycle controllers.

Normalize destination names case-insensitively and without `.bsp` only for matching. Retain original spelling in source.

### 3. Form the alignment hypothesis

For map A and map B with matching landmark `L`:

```text
T_initial = origin_A(L) - origin_B(L)
```

Do not apply this immediately. Score it against transition-neighborhood evidence:

- coplanar floor/ceiling planes;
- matching opening outlines;
- transformed brush-volume overlap;
- material and texture-axis agreement;
- props and decals near the seam;
- changelevel/transition-volume correspondence;
- player-hull continuity;
- collision-free approach paths.

The initial release allows a translation only. Rotation and reflection candidates are disabled.

### 4. Select a seam volume

Construct a conservative seam region from the union of:

- both changelevel triggers;
- both transition volumes;
- geometry within a profile-configured expansion distance;
- reachable empty cells connected to the transition opening.

Only geometry inside this seam volume can be considered duplicated transition buffer data. Map-wide similarity is never treated as a deletion signal.

### 5. Classify overlap

Every transformed B solid intersecting A is classified:

1. exact semantic duplicate;
2. plane-equivalent convex volume;
3. compatible touching geometry;
4. partial overlap with identical visible boundary;
5. conflicting world geometry;
6. world/brush-entity conflict;
7. unsupported displacement overlap;
8. numerically ambiguous.

Automatic deletion is initially limited to classes 1 and qualified class 2 inside the seam volume. Conflicts block acceptance and appear in the review report.

### 6. Allocate IDs and namespace names

- Preserve A IDs when legal.
- Allocate all imported B IDs from a collision-free deterministic range.
- Prefix B map-local targetnames.
- Rewrite every typed target reference, parent, filter, path link, I/O target, and supported script reference in one transaction.
- Preserve Source special names and explicitly declared shared/global names.
- Reject unresolved affected references.

### 7. Reconcile world and singleton systems

Create a conflict table for:

- `worldspawn` keys;
- sky and 3D skybox;
- light environments;
- fog, tonemap, color correction;
- soundscapes and ambient systems;
- cubemaps;
- chapter/title/autosave logic;
- global states and globalnames;
- map scripts and map Lua;
- nav/AI networks;
- spawn points and game rules;
- water and detail controllers.

Each class has a versioned policy: choose A, choose B, spatially switch, synthesize a controller, preserve both, or block.

### 8. Synthesize region lifecycle

Replace cross-level load semantics with an explicit region state machine. See [Campaign lifecycle](CAMPAIGN_LIFECYCLE.md).

### 9. Materialize a generated VMF

- Apply source-preserving edits to A.
- Append transformed and namespaced B objects using deterministic formatting.
- Keep all original inputs unchanged.
- Emit a patch manifest and a provenance map from every output object to its source.

### 10. Validate

Run static, compiler, metric, and runtime gates. The stitched result is accepted only when every mandatory scenario passes.

## Capacity planning

Before generating a candidate, estimate and then compile-measure:

- world coordinate extents;
- solids, sides, planes, vertices, faces;
- entities and networked entity risk;
- overlays and cubemaps;
- displacements;
- nodes, leaves, clusters, portals, areas, areaportals;
- visibility and lighting lump sizes;
- static prop and detail data;
- packed asset size;
- expected simultaneous NPC/physics/script load.

Limits come from the exact game profile and qualification probes. Stock SDK values are informational only for a current GMod Tools++ profile.

## Acceptance criteria

A two-map stitch is accepted when:

- both source files round-trip exactly;
- alignment confidence is proven by configured geometric checks;
- no unsupported overlap remains;
- all changed references resolve uniquely;
- generated solids satisfy geometry invariants;
- VBSP/VVIS/VRAD complete under the selected quality tier;
- no new compiler errors or unapproved warnings appear;
- BSP budgets remain below safety margins;
- forward transition, reverse traversal, save/load, death/reset, doors, scripts, NPCs, audio, fog, and sky scenarios pass;
- review artifacts explain every mutation.
