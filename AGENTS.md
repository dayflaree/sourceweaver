# Agent instructions

Read `docs/AUTOMATION_CONTRACT.md`, `docs/ARCHITECTURE.md`, and the matching skill under `skills/` before editing.

Non-negotiable rules:

- preserve source VMFs byte-identically outside explicit edits;
- never overwrite input maps;
- use FGD/schema types for semantic rewrites;
- keep AI outside correctness authority;
- bind results to exact compiler/runtime fingerprints;
- add a synthetic fixture and failure test for every transformation;
- do not commit game content or compiler binaries;
- report validation actually run and any runtime limitation.
