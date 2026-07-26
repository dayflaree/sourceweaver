# Automation contract

## Objective

A mapper should provide VMFs, select a target game profile, state the desired result, and review a generated report. SourceWeaver performs all repetitive analysis, transformation, compilation, comparison, packaging, and test execution.

## Meaning of “automatic”

An operation is automatic only when all of these are true:

1. Inputs are immutable and fingerprinted.
2. Source parsing is lossless.
3. Every changed field has a typed transformation rule.
4. Geometry edits pass exact structural invariants.
5. The transformed VMF compiles with the exact target compiler set.
6. BSP/PRT/lump metrics satisfy configured acceptance thresholds.
7. Runtime scenarios complete without new errors or behavior regressions.
8. A reversible patch manifest explains every mutation.

A tool-generated guess that compiles once is a proposal, not an accepted automatic result.

## Confidence classes

| Class | Meaning | Default action |
|---|---|---|
| Proven | Deterministic invariant or compiler/runtime result verifies the claim | May auto-apply |
| High | Multiple independent signals agree; final compiler/runtime proof pending | Generate candidate and validate |
| Medium | Plausible pattern with unresolved semantics | Require review before expensive validation |
| Low | Heuristic or AI-only inference | Report only |
| Unknown | Unsupported data, branch, class, or behavior | Block transformation |

## Non-destructive rules

- Never overwrite an input VMF.
- Work in a generated directory named by input hash and run ID.
- Emit a text patch and a structured manifest.
- Preserve untouched bytes exactly.
- Keep a baseline compile beside every candidate compile.
- Reject overlapping source edits.
- Retain compiler logs, command lines, fingerprints, and metric files.
- Make output acceptance atomic: all gates pass or no accepted artifact is produced.

## Supported-envelope principle

SourceWeaver does not claim universal correctness. Each feature declares a support envelope. For example, the first stitcher supports two directly connected, translation-aligned, original VMFs with known FGD data and no unresolved script-controlled transition state. A map outside that envelope receives an explicit blocker report.

The envelope expands only after adding:

- a legal regression fixture;
- a deterministic transformation rule;
- compile validation;
- runtime validation;
- documentation;
- a recorded acceptance threshold.

## Manual work policy

Allowed human work:

- choose inputs and target profile;
- specify product intent;
- review conflicting design choices;
- approve or reject a proven patch;
- provide a legal minimal fixture for unsupported behavior.

The pipeline must handle routine cleanup, retries with deterministic candidate variants, compiler execution, log triage, metrics, reports, and rollback.

## Refusal conditions

The tool refuses automatic mutation when it encounters:

- parse or round-trip failure;
- unknown editor extensions inside a span that must be rewritten;
- ambiguous coordinate-bearing keyvalues;
- unresolved targetname references affected by a namespace operation;
- non-convex or numerically unstable generated solids;
- compiler fingerprint changes without requalification;
- leaked world or areaportal;
- map limit or world-bound risk;
- runtime crash, hang, new error, or scenario divergence;
- a legal/provenance policy violation.
