---
name: sourceweaver-vmf-integrity
description: Safely parse, preserve, and patch VMF/Hammer++ text without losing unknown data or duplicate keys.
---

# VMF integrity

## Trigger

Use for lexer/parser work, encoding, CST nodes, source edits, serialization, unknown blocks, Hammer++ precision, or round-trip defects.

## Required inputs

- smallest legal synthetic VMF reproducer;
- expected encoding and newline form;
- exact source span and semantic object involved;
- whether the object contains unknown/editor-specific data.

## Procedure

1. Hash and retain the original bytes.
2. Lex every source character into a span-bearing token, including trivia.
3. Parse ordered entries; never collapse them into a dictionary.
4. Render without edits and assert byte identity.
5. Map semantic fields back to unique CST spans.
6. Plan non-overlapping edits with old-text hashes.
7. Preserve original quoting, separator, encoding, and line ending where possible.
8. Reparse the patched result and re-run byte-preservation checks outside changed spans.
9. For geometry-bearing Hammer++ extensions, require fixture-backed transformation rules.
10. Add success, malformed-input, duplicate-key, non-ASCII, and unknown-block tests.

## Blockers

- round-trip mismatch;
- ambiguous source span;
- unknown nested data on an object requiring reconstruction;
- overlapping edits;
- lossy encoding conversion;
- malformed braces/strings;
- parser recursion or size limits exceeded.

## Acceptance gates

- unedited output equals input bytes;
- 100% of untouched spans are unchanged;
- duplicate output ordering remains intact;
- unknown data survives;
- the patched file reparses;
- semantic delta equals the manifest.

## References

- `docs/VMF_INTEGRITY.md`
- `docs/PATCH_AND_PROVENANCE.md`
- `docs/adr/0001-lossless-cst.md`
