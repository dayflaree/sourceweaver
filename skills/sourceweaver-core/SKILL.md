---
name: sourceweaver-core
description: Build or review SourceWeaver features while preserving non-destructive, compiler-verified automation guarantees.
---

# SourceWeaver core

## Trigger

Use for any repository change spanning multiple subsystems, new commands, schemas, orchestration, or architectural decisions.

## Mandatory rules

1. Read `docs/AUTOMATION_CONTRACT.md` and `docs/ARCHITECTURE.md`.
2. Never overwrite user inputs.
3. Preserve untouched VMF bytes exactly.
4. Use typed transformations and immutable manifests.
5. Keep AI outside correctness authority.
6. Bind compiler/runtime claims to exact fingerprints.
7. Block unsupported conditions instead of guessing.
8. Add synthetic legal fixtures; never add game content or compiler binaries.

## Procedure

1. Define the feature's support envelope and explicit exclusions.
2. List deterministic invariants and failure responses.
3. Identify CST, semantic, geometry, compiler, runtime, and report impacts.
4. Add or update stable schemas before implementation.
5. Implement a pure planning stage and separate materialization stage.
6. Add unit, failure, property, golden, and integration tests as applicable.
7. Run `ruff check .`, `mypy`, and `pytest --cov`.
8. Run compiler/runtime qualification when behavior or geometry changes.
9. Update research ledger, architecture decisions, roadmap, and definition of done.
10. Report exactly what was validated and what remains disabled.

## Acceptance gates

- source round-trip corpus passes;
- no direct free-form AI-to-VMF path exists;
- generated artifacts have provenance and rollback;
- every unknown state has a deterministic block/review rule;
- CI passes;
- feature remains disabled by default until qualification gates pass.

## Outputs

- code and tests;
- updated docs/ADR;
- support-envelope declaration;
- report/schema changes;
- validation evidence.
