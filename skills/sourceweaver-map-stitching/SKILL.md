---
name: sourceweaver-map-stitching
description: Stitch directly connected Source VMFs using landmark alignment, conservative overlap handling, typed namespacing, and region lifecycle synthesis.
---

# Map stitching

## Trigger

Use for transition graph extraction, map alignment, seam selection, duplicate transition geometry, merged IDs/names, world controller reconciliation, or region lifecycle.

## Preconditions

- original VMFs preferred;
- exact profile and FGDs resolved;
- both files pass lossless round-trip;
- compiler/runtime harness available for final acceptance;
- initial support tier requires one direct edge and translation-only alignment.

## Procedure

1. Build transition records from `trigger_changelevel`, `info_landmark`, and `trigger_transition`.
2. Form landmark translation hypothesis.
3. Verify floor/ceiling/opening planes, reachable space, trigger volumes, materials, and collision continuity.
4. Define a bounded seam volume.
5. Transform B in memory and classify every intersection inside and outside the seam.
6. Remove only qualified exact/plane-equivalent duplicates inside the seam.
7. Allocate deterministic collision-free IDs.
8. Namespace B map-local names and atomically rewrite typed references.
9. Produce a singleton/world/controller conflict matrix.
10. Apply versioned reconciliation policies.
11. Synthesize region lifecycle and activation order.
12. Run capacity preflight.
13. Materialize a generated VMF and provenance manifest.
14. Compile baseline/candidate and run all mandatory runtime scenarios.
15. Accept atomically or reject the complete candidate.

## Blockers

- multiple ambiguous transition edges/landmarks;
- rotation/reflection required in initial tier;
- unresolved overlap or displacement seam;
- ambiguous reference rewrite;
- opaque transition script;
- singleton conflict without policy;
- capacity margin exceeded;
- any mandatory compiler/runtime failure.

## Outputs

- stitched generated VMF;
- patch/provenance manifest;
- alignment and seam evidence;
- conflict/reconciliation report;
- baseline/candidate metrics;
- runtime scenario results.

## References

- `docs/MAP_STITCHING.md`
- `docs/CAMPAIGN_LIFECYCLE.md`
- `docs/DEFINITION_OF_DONE.md`
