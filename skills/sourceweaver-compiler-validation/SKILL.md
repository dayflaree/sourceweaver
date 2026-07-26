---
name: sourceweaver-compiler-validation
description: Discover, fingerprint, qualify, invoke, and interpret Source/GMod compilers reproducibly and safely.
---

# Compiler validation

## Trigger

Use for compiler profiles, process execution, logs, BSP/PRT inspection, limits, metrics, caching, or qualification.

## Procedure

1. Discover local tools through the selected game profile.
2. Hash every executable and record size/version metadata.
3. Refuse acceptance use for an unqualified fingerprint.
4. Qualify the hash against sealed, leak, portal, hint, instance, displacement, format, and resource fixtures.
5. Invoke through argument arrays in an isolated work directory.
6. Capture exact command, environment, stdout, stderr, timeout, and process result.
7. Kill the process tree on timeout or cancellation.
8. Normalize known log messages and flag unknown error-like output.
9. Inspect BSP/PRT/lumps; do not trust exit code alone.
10. Compile a same-environment baseline for every candidate.
11. Compare metrics and warnings.
12. Store immutable artifacts keyed by all inputs/tool/profile hashes.

## Blockers

- missing compiler for required tier;
- changed/unqualified hash;
- process timeout/crash;
- leak/limit/fatal/unknown error-like output;
- malformed or missing output artifact;
- nondeterministic critical lumps;
- profile/tool mismatch.

## References

- `docs/COMPILER_VALIDATION.md`
- `docs/GAME_PROFILES.md`
- `docs/FAILURE_MODES.md`
