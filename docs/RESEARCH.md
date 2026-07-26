# Research report

Research baseline: **July 26, 2026**.

## Conclusion

The project is feasible as a compiler-backed VMF engineering system. Parsing VMF text is straightforward. Reliable automation depends on preserving editor data, reconstructing valid brush geometry, understanding FGD-typed entity semantics, and proving each optimization with the exact target compilers and game runtime.

The architecture therefore separates:

- a lossless concrete syntax tree for source preservation;
- a semantic Source model for entities, I/O, instances, assets, and BSP data;
- an exact geometry kernel for committed modifications;
- approximate spatial models for candidate discovery;
- compiler and runtime harnesses for final authority;
- AI for intent, ranking, explanations, and ambiguity management.

## Existing foundations

### Hammer++

Hammer++ is a modernized Source 1 editor with improved stability, brush accuracy, previews, instances, compile workflow, and many editor improvements. Its official feature documentation says it stores extra precision to reduce vertex drift while retaining traditional plane data for compiler compatibility.

Implication: SourceWeaver should be an external companion and must preserve unknown/editor-specific data.

### srctools

`srctools` supports VMF, BSP, FGD, VPK, VMT, VTF, instances, entity I/O, and other Source formats. It is the strongest semantic library foundation. Direct source inspection at commit `5a8ed668f34693f59f26cc04ff367788ed738f8b` showed that `VMF.parse()` builds known semantic objects and `VMF.export()` reconstructs known top-level blocks. This is appropriate for compiler transformations but does not constitute lossless preservation of unknown VMF extensions.

Implication: use `srctools` behind SourceWeaver’s own lossless syntax layer.

### HammerAddons

HammerAddons demonstrates large-scale compiler and postcompiler transformations. Its areaportal transform also shows conservative class-based handling: it modifies known fade-target classes and skips unknown behavior.

Implication: follow the same safety pattern and integrate rather than duplicate mature semantics where practical.

### CompilePal and VMFInstanceInserter

CompilePal demonstrates user-friendly compile orchestration and extensible steps. VMFInstanceInserter demonstrates a non-destructive temporary-VMF preprocessing model.

Implication: generated worktrees and compiler steps are established Source workflows.

### BSPSource

BSPSource can reconstruct VMFs from BSPs, but decompilation is not perfect. Compiler-consumed constructs and original editor intent may be absent or approximated.

Implication: original VMFs receive the highest support class. Decompiled inputs remain analyzable but cannot receive identical confidence.

### Existing automatic optimization experiments

VMF-Optimizer attempts ray-based automatic face visibility and nodraw application. Its own documentation limits accuracy and scale.

Implication: ray sampling can nominate candidates; it cannot prove invisibility for arbitrary maps. SourceWeaver needs visibility/PVS/runtime proof and conservative exceptions.

## Current GMod and Tools++ findings

The installed GMod toolchain inspected during research contained 64-bit:

- `vbspplusplus.exe`
- `vvisplusplus.exe`
- `vradplusplus.exe`
- `bspzipplusplus.exe`

The GMod preview changelog also announces 64-bit Hammer/builds and compile-tool changes including VBSP `-embed` and BSPZIP repacking/compression.

Hammer++’s official Tools page, updated in June 2026, documents:

- VBSP++ optimizations and substantially raised or configurable soft limits;
- areaportal leak testing;
- support for several BSP/static-prop formats;
- VVIS++ producing the same visibility result as stock VVIS with substantially faster execution;
- `func_viscluster` deprecation under VBSP++/VVIS++;
- VRAD++ branch and lighting improvements;
- BSPZIP++ support for GMod compression;
- a requirement to ask permission before redistributing these compilers.

Implications:

1. Stock SDK 2013 limit tables cannot be treated as GMod’s definitive current limits.
2. Compiler profiles must be tied to exact executable hashes.
3. The tool should discover locally installed compilers and never vendor them.
4. `func_viscluster` generation is not part of the current GMod target plan.
5. Hints and areaportals still affect the compiled result even when VVIS runs faster.

## Areaportal semantics verified in source

Valve SDK 2013 `vbsp/portals.cpp` flood-fills reachable empty leaves into areas and treats areaportal nodes as boundaries. The compiler records the areas touching each areaportal, warns when one touches more than two areas, and reports a leak when it does not touch two areas.

Implication: doorway shape and brush adjacency alone are insufficient. Candidate proof requires an empty-space cut that yields exactly two valid areas after VBSP.

## Level-transition semantics verified in source

Valve SDK 2013 `triggers.cpp` shows that:

- `trigger_changelevel` identifies a destination and landmark;
- the matching `info_landmark` supplies the transition origin;
- candidate entities are found from the landmark PVS;
- named `trigger_transition` volumes screen those entities;
- entity capabilities/global names determine save eligibility;
- a transition list has a 512-entity cap in this SDK branch.

The current GMod preview changelog still mentions level-transition fixes, confirming that transition behavior remains relevant.

Implication: landmarks provide alignment evidence, while a one-BSP merge must replace the entire load/save/lifecycle effect rather than merely delete `trigger_changelevel`.

## VMF and entity-I/O details

Entity outputs can be repeated keys. Source branches use comma-separated or ESC-separated output fields. Instance fixups, special targetnames beginning with `!`, global names, and wildcard searches complicate renaming.

Implication: a merge must preserve duplicate keys and rewrite only FGD-typed references. Raw global search/replace is unsafe.

## Map merging feasibility

A deterministic adjacent-map stitcher is feasible when:

- both source maps are available as VMF;
- the transition edge and matched landmarks are unambiguous;
- alignment is translation-only in the initial support tier;
- overlapping transition geometry can be classified safely;
- targetnames and IDs can be namespaced with typed rewrites;
- map-level controllers can be reconciled;
- region lifecycle behavior can be synthesized;
- the resulting map remains within target branch limits and world bounds;
- compile and runtime scenarios pass.

Whole-campaign flattening is not a single operation. It is constrained by world extent, BSP lumps, networked entities, scripting, lighting, AI systems, and simultaneous region state. Campaign-scale support should use measured partitions or region streaming concepts only after two-map stitching is proven.

## Visibility optimization feasibility

### Nodraw

Safe candidates include fully occluded internal faces or sealed inaccessible void surfaces. Sampling alone does not prove safety because cameras, mirrors, portals, breakables, moving entities, and scripted views can expose surfaces.

### func_detail

Candidates must be non-sealing decorative world brushes that do not define critical visibility, areaportal, water, or collision behavior. Compile comparison is required.

### Areaportals

Use an empty-space connectivity graph to nominate narrow graph cuts. Fit a portal brush against exact sealing planes, then require VBSP area proof and PVS/runtime benefit.

### Hints

Hint placement is a bounded search problem. Candidate planes should derive from existing structural planes. Keep a candidate only when compile metrics and sampled runtime visibility improve without unacceptable leaf/PVS growth.

## Correct role of AI

AI can interpret user intent, rank candidates, explain compiler output, classify likely entity roles, and synthesize review reports. AI cannot be the authority for convexity, sealing, target resolution, compile acceptance, or performance improvement.

## Research outcome

No unresolved assumption blocks the source-integrity, analysis, compiler-discovery, or test-harness phases. Advanced stitch and optimization phases still require empirical qualification against a legal fixture corpus. Those are experiments with explicit pass/fail gates, rather than assumptions hidden in the design.
