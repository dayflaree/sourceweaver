# VMF integrity

## Why semantic export is insufficient

A VMF editor-safe transformer must retain data it does not understand. Reconstructing only known structures can lose editor extensions, comments, exact duplicate ordering, formatting, or future fields.

SourceWeaver uses two representations:

- a lossless CST as the write authority;
- a semantic IR as the analysis authority.

## Required invariants

For every accepted input:

```text
read bytes
 -> decode using recorded encoding
 -> lex and parse
 -> render without edits
 -> encode using recorded encoding
 == original bytes
```

Failure is a blocker.

## Encoding

Initial policy:

- honor UTF-8 BOM;
- accept valid UTF-8;
- fall back to Windows-1251, matching common Source tooling behavior;
- retain original encoding on output;
- retain CRLF or LF;
- never normalize line endings implicitly.

Future support may add explicit encoding declarations and byte-level token spans.

## Duplicate keys

Entity outputs commonly repeat the same key. The CST stores entries as an ordered sequence, never a dictionary. Semantic indexes may group duplicates while retaining source identity and order.

## Unknown data

Unknown blocks and pairs are preserved. A transformation may proceed when unknown data lies outside every changed span and has no known dependency on the changed object. Unknown data nested inside an object requiring rewrite marks that object unsupported until a preservation rule exists.

## Hammer++ precision

Public examples show a `vertices_plus` block under a side with repeated `v` entries. SourceWeaver treats it as editor-owned geometry data until qualified against real legal fixtures.

Rules:

- untouched sides retain the block byte-identically;
- translated sides may translate each recognized precise vertex while retaining ordering and formatting where possible;
- rotated or reconstructed sides regenerate both plane and precise-vertex data only after a Hammer++ reopen/save qualification test;
- a side with an unknown precision extension is not automatically reconstructed.

## Stable object identity

VMF numeric IDs are mutable and can collide during merges. SourceWeaver assigns an internal UUID derived from:

```text
source file hash + source path + syntactic object span + original VMF ID
```

Generated objects receive random or content-derived UUIDs and new legal VMF IDs allocated only at materialization.

## Patch safety

Each edit records:

- source span;
- old-text hash;
- replacement;
- object UUID;
- transformation rule;
- reason and evidence;
- expected semantic delta.

Before applying, the planner verifies the old-text hash and rejects overlapping edits.

## Qualification fixture categories

- stock Hammer VMF;
- Hammer++ VMF with precision blocks;
- repeated entity outputs;
- escaped output separators;
- comments in every legal position;
- non-ASCII keyvalues;
- displacements and multiblend data;
- instances and fixups;
- hidden entities, groups, visgroups, cameras, cordons;
- unknown top-level and nested blocks;
- malformed/partial VMFs with expected errors.
