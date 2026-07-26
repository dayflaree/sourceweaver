---
name: sourceweaver-visibility
description: Generate and validate conservative nodraw, func_detail, areaportal, hint/skip, and occluder candidates through compile-in-loop measurement.
---

# Visibility automation

## Trigger

Use for visibility analysis or optimization transformations.

## Procedure

1. Compile an untouched full baseline with exact tool fingerprints.
2. Extract BSP/PVS topology and deterministic runtime sample metrics.
3. Generate candidates from geometry/spatial evidence.
4. Exclude dynamic views, breakables, moving geometry, special materials, scripts, and unknown classes.
5. Fit committed geometry against exact structural planes.
6. Test one candidate per isolated experiment first.
7. Reject any leak, warning, topology mismatch, or correctness regression.
8. For areaportals, require exact two-area compiler proof and door scenario checks.
9. For hints, run bounded full-VVIS candidate/combination search.
10. For nodraw/detail, run visual/collision/scripted-view scenarios.
11. Compare paired repeated runtime metrics.
12. Select a Pareto-qualified set and emit rejected-alternative reasons.

## Never do

- infer areaportal validity from brush adjacency alone;
- apply skip as a general surface optimization;
- accept ray sampling as proof of invisibility;
- generate `func_viscluster` for the current GMod Tools++ profile;
- accept faster compile time as sufficient runtime benefit;
- waive correctness for performance.

## Acceptance gates

- source/geometry invariants pass;
- full compiler tier passes;
- relevant BSP topology matches expected;
- runtime correctness scenarios pass;
- repeated metrics exceed practical-improvement threshold;
- leaf/PVS/portal growth remains inside policy.

## References

- `docs/VISIBILITY_OPTIMIZATION.md`
- `docs/METRICS.md`
