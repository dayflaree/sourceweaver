# Architecture

## Design principles

1. Preserve source before understanding it.
2. Separate candidate discovery from committed geometry.
3. Type every semantic rewrite.
4. Make transformations pure and replayable.
5. Use the target compiler and runtime as final authorities.
6. Record evidence, confidence, and provenance.
7. Disable unsupported behavior rather than guessing.

## Layers

```text
CLI / desktop UI / Hammer compile-step adapter
                    |
              Intent and policy
                    |
        +-----------+-----------+
        |                       |
  Lossless VMF CST         Asset/source resolver
        |                       |
        +---------- Semantic IR + FGD profile
                            |
             +--------------+--------------+
             |                             |
       Geometry kernel               Entity/I/O graph
             |                             |
       Empty-space graph             Lifecycle graph
             +--------------+--------------+
                            |
                 Candidate generators
                            |
                 Typed patch planner
                            |
                  Static invariant gates
                            |
         Fingerprinted compiler experiment harness
                            |
              BSP/PRT/lump metric extraction
                            |
                 GMod runtime scenarios
                            |
                Acceptance + review report
```

## Lossless concrete syntax tree

The CST stores exact token spans for:

- whitespace;
- comments;
- quoted strings and escape spelling;
- bare atoms;
- braces;
- duplicate keys;
- unknown blocks;
- source encoding and line endings.

Changes are source-span edits. Unchanged spans are copied exactly.

## Semantic intermediate representation

The semantic model does not replace the CST. It references CST nodes and carries:

- stable source IDs and generated UUIDs;
- original map/provenance;
- entity classname and FGD definition;
- typed properties;
- outputs and special target references;
- brush solids, sides, planes, materials, displacement data;
- groups, visgroups, instances, cordons, cameras;
- unknown/unsupported flags;
- transformation eligibility.

## Geometry services

- normalized plane representation;
- convex polyhedron reconstruction;
- robust orientation and sidedness predicates;
- face polygon clipping;
- coplanar overlap;
- brush intersection/containment;
- adjacency and seam graphs;
- transform propagation;
- generated-solid validation;
- adaptive spatial partitioning for empty-space discovery.

Exact and approximate geometry use separate types so approximate candidate data cannot accidentally become committed VMF geometry.

## Entity and logic services

- FGD inheritance and keyvalue types;
- targetname index;
- entity I/O graph with duplicate outputs preserved;
- instance fixup scopes;
- special names such as `!activator`, `!caller`, and wildcards;
- parent/child graph;
- global state and globalname analysis;
- resource path resolution;
- script and map-Lua risk classification;
- coordinate/angle transform registry.

## Experiment harness

Each experiment consists of:

- immutable baseline inputs;
- profile and executable fingerprints;
- one deterministic patch plan;
- generated VMF;
- compiler commands and environment;
- compiler outputs/logs;
- BSP/PRT analysis;
- optional runtime scenarios;
- objective function;
- verdict and rollback data.

## Storage

Use a content-addressed work directory:

```text
.work/
  <source-set-hash>/
    baseline/
    candidates/<candidate-id>/
    accepted/
    reports/
    cache/
```

SQLite or an equivalent embedded database indexes experiments. Large generated artifacts stay outside Git.

## Process isolation

Compiler and runtime commands execute as child processes with:

- explicit working directories;
- sanitized environment;
- bounded duration;
- captured stdout/stderr;
- process-tree termination;
- immutable command manifests;
- no shell interpolation for user-supplied paths.

## User interfaces

Initial surfaces:

1. CLI for deterministic automation and CI.
2. Local report UI for geometry/logic diffs.
3. Hammer++/CompilePal integration through external commands.

No undocumented in-process Hammer++ hooking is required.
