---
name: sourceweaver-release
description: Prepare a reproducible SourceWeaver release and GitHub delivery without shipping proprietary assets or unqualified automation.
---

# Release

## Trigger

Use for versioning, release candidates, packaging, GitHub releases, or enabling a feature by default.

## Procedure

1. Freeze source commit and dependency lock/constraints.
2. Run public CI matrix.
3. Run lossless fixture corpus.
4. Run every enabled transformation's compiler qualification on supported fingerprints.
5. Run required GMod runtime suites.
6. Verify schemas and backward-compatibility policy.
7. Audit repository/package for maps, BSPs, assets, binaries, paths, and secrets.
8. Update research baseline and stale mutable claims.
9. Generate reproducibility and validation reports.
10. Version, tag, build, inspect package contents, and publish.

## Feature enablement gate

A feature can become default only when its definition of done passes on its declared support envelope and held-out fixtures show no false auto-accepts.

## Forbidden release contents

- Valve or third-party maps/assets;
- Hammer++/Tools++ or Source compiler binaries;
- local absolute paths or private project metadata;
- unqualified compiler fingerprints marked trusted;
- claims of universal correctness.

## References

- `docs/DEFINITION_OF_DONE.md`
- `docs/LEGAL_AND_DISTRIBUTION.md`
- `docs/TEST_STRATEGY.md`
