---
name: sourceweaver-research-maintenance
description: Verify current Source/GMod/Hammer++ behavior and update the project evidence ledger without turning mutable claims into assumptions.
---

# Research maintenance

## Trigger

Use for current tool behavior, engine branches, documentation, limits, licensing, library APIs, or an unfamiliar Source feature.

## Source priority

1. exact local executable/source observation;
2. first-party source code or official documentation;
3. primary project repository and release notes;
4. respected technical references;
5. community discussion as a lead only.

## Procedure

1. State the precise claim being verified.
2. Search current web sources and compare publication/update dates.
3. Fetch primary source or inspect the exact code/binary where available.
4. Record source URL, date, commit/hash, and relevant branch.
5. Separate fact, local observation, engineering inference, and experiment.
6. Add/update `docs/RESEARCH_LEDGER.md` and `docs/SOURCE_INDEX.md`.
7. Convert branch-dependent claims into profile data.
8. Add a qualification experiment for anything compiler/runtime dependent.
9. Avoid copying third-party code or content without license review.
10. Mark stale claims and invalidate affected cached qualification.

## Confidence labels

- Proven: deterministic test, exact source, or exact local artifact observation.
- High: first-party documentation with branch/date caveat.
- Medium: multiple consistent signals needing empirical proof.
- Low: heuristic/community claim.
- False: contradicted by primary evidence.

## Output

A concise finding, citations/links, ledger entry, design consequence, and required experiment when confidence is below proven.
