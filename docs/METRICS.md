# Metrics and acceptance objectives

## Rule

A metric must be reproducible, attributable, and compared under the same profile. Single FPS readings are insufficient.

## Compiler metrics

- stage wall-clock and CPU time;
- peak working set where observable;
- solid/side/plane/vertex/face/node/leaf/cluster/portal counts;
- area and areaportal counts;
- PVS/PAS and other relevant lump sizes;
- lightmap and static-prop data size;
- BSP and packed pak size;
- warnings by normalized code.

## Runtime metrics

At deterministic sample points and routes:

- frame time distribution (median, P95, P99);
- CPU/GPU frame components where exposed;
- visible leaves/clusters;
- visible world faces;
- visible static/dynamic props;
- draw calls and triangles where exposed;
- entity count, thinking entities, NPC count;
- memory and load time;
- portal open/closed deltas.

## Correctness metrics

- source byte preservation percentage outside changed spans: 100%;
- resolved affected references: 100%;
- static invariant failures: 0;
- compiler fatal errors/leaks: 0;
- new forbidden warnings: 0;
- mandatory runtime assertions passed: 100%;
- unexplained semantic changes: 0.

## Budgets

Each profile defines hard limits and softer safety margins. Candidates are rejected before the hard limit and normally before the safety margin. User policy may tighten margins but cannot waive engine hard limits.

## Statistical comparison

- Warm up before sampling.
- Use repeated route runs.
- Compare paired samples when possible.
- Record hardware, resolution, graphics settings, branch, and mounted content.
- Report confidence intervals or robust spread.
- Require a minimum practical improvement to avoid accepting noise.

## Multi-objective selection

A candidate set must be Pareto-superior or meet a declared tradeoff policy. Example: a small compile-time increase may be accepted for a large P95 frame-time reduction, while leaf explosion or BSP budget risk remains forbidden.

## Regression policy

Correctness gates dominate performance metrics. A faster candidate with any behavior regression is rejected.
