# Research and confidence ledger

The ledger separates verified facts, engineering conclusions, and experiments still required. “Proven” means supported by first-party source, local executable inspection, or a deterministic repository test. It does not mean every future game branch will behave identically.

| Claim | Evidence | Confidence | Design consequence |
|---|---|---:|---|
| VMF can contain repeated keys and arbitrary/unknown blocks | VMF/KeyValues behavior; entity output structures; lossless fixture tests | Proven | Preserve concrete syntax and duplicates |
| VMF side planes can reconstruct simple convex brush geometry through half-space intersection and classify tested convex brush relations | Synthetic cube/open/sliver geometry tests plus equality/containment/touching/overlap/disjoint relation tests in `tests/test_geometry.py` and `tests/test_analysis.py` | Proven for the tested read-only slice | Surface invalid brush blockers and relation evidence; keep transformation/removal/compiler authority disabled |
| `srctools` is a broad Source semantic foundation | Project docs and source at `5a8ed66...` | Proven | Use it as semantic adapter |
| `srctools.VMF.export()` reconstructs known structures | Direct source inspection | Proven | Do not use it as sole editor-safe round-trip writer |
| Hammer++ stores additional precision information | Hammer++ official features; public `vertices_plus` examples | High | Preserve unknown side subblocks; add real Hammer++ fixture qualification |
| Current GMod install includes Tools++ compilers | Local file and PE inspection on 2026-07-26 | Proven for inspected install | Discover and hash exact local executables |
| GMod’s current/upcoming branch includes 64-bit Hammer/build tools | Facepunch preview changelog | High/current branch dependent | Maintain stable/dev profile separation |
| VVIS++ claims same output as stock VVIS | Hammer++ official Tools page | High; requires fixture comparison for each hash | Add differential qualification before trusting a new binary |
| VBSP++ deprecates `func_viscluster` | Official Tools page and inspected executable strings | High | Do not generate visclusters for GMod Tools++ profile |
| VBSP area flood requires each areaportal to touch two areas | Valve SDK `portals.cpp` | Proven for SDK 2013 lineage | Use exact two-area compile gate |
| Areaportal adjacency alone proves correctness | Contradicted by compiler flood algorithm | False | Never place from adjacency alone |
| Transition entities are selected from landmark PVS and transition volumes | Valve SDK `triggers.cpp` | Proven for SDK 2013 lineage | Model transition state and volumes |
| SDK 2013 transition list cap is 512 | Valve SDK `triggers.cpp` | Proven for that branch | Treat as source-branch fact, probe/profile GMod separately |
| Landmark origins are enough to align maps | Landmark translation is valid evidence; overlap may still conflict | Medium | Run geometric alignment verification |
| Arbitrary whole campaigns can always fit one BSP | Engine limits and world bounds contradict this | False | Capacity planner and partitioning required |
| Raycasting proves a face is never visible | Cameras/scripts/dynamic geometry contradict this | False | Use rays as candidate evidence only |
| A successful compile proves gameplay equivalence | Runtime logic can still diverge | False | Mandatory runtime scenario layer |
| Decompiled VMF is semantically identical to original | BSPSource documents reconstruction limits | False | Lower confidence and disable some auto-mutations |
| All numeric triplets can be translated as coordinates | Colors and arbitrary parameters contradict this | False | FGD/schema-typed transformations only |
| Compiler limits are static across Source branches | Tools++ supports different/configurable limits | False | Exact executable profiles and probes |
| Compiler binaries may be redistributed with this repository | Tools++ page asks for permission | False absent permission | Discover local tools; never vendor binaries |

## Qualification experiments still required

These are scheduled work items, not hidden assumptions:

1. Round-trip a diverse legal Hammer++ fixture set and verify byte identity.
2. Compare VVIS and VVIS++ output on controlled maps for each adopted executable fingerprint.
3. Build synthetic areaportal fixtures covering valid cuts, bypass leaks, exterior leaks, and more-than-two-area cases.
4. Build synthetic hint fixtures and verify metric extraction.
5. Build direct-transition fixture pairs with duplicate geometry and conflicting targetnames.
6. Build runtime scenarios for logic lifecycle, backtracking, doors, NPCs, soundscapes, fog, and skybox transitions.
7. Probe target branch limits through generated fixtures instead of copying a generic table.
8. Validate GMod stable and dev branches independently.

A feature remains disabled by default until its qualification suite passes.
