# Test strategy

## Test pyramid

### Unit tests

- lossless lexer/parser;
- encoding and line endings;
- duplicate keys and output separators;
- source edit conflicts;
- plane normalization and robust predicates;
- FGD type resolution;
- targetname rewriting;
- patch/provenance serialization;
- log parsing and metric extraction.

### Property and fuzz tests

- arbitrary whitespace/comments around legal KeyValues structures;
- parse/render byte identity;
- edits preserve every untouched source span;
- convex brush reconstruction under permutations;
- transform/inverse-transform consistency;
- namespace operations preserve graph connectivity;
- malformed input fails safely without hangs or memory explosion.

### Golden fixtures

Synthetic, redistributable VMFs cover:

- minimal sealed room;
- world leak;
- valid/invalid areaportals;
- hint patterns;
- doors and portal linkage;
- displacements;
- instances and fixups;
- entity-I/O duplicates and cycles;
- transition pair with duplicate seam;
- conflicting controllers;
- lifecycle/backtracking;
- profile limits.

### Compiler integration

Each qualified fingerprint compiles the golden corpus. Expected normalized messages and BSP topology are versioned.

### Runtime integration

GMod scenarios exercise generated maps and emit machine-readable results.

### Differential tests

- SourceWeaver semantic adapter versus `srctools` on supported fields;
- stock VVIS versus VVIS++ where available;
- baseline versus candidate BSP lumps;
- Hammer++ open/save comparison for generated precision data.

## CI tiers

Public CI runs tests that need no proprietary game installation. Local/release qualification runs compiler and GMod suites when the owner provides installed tools.

## No copyrighted fixtures

Tests use original synthetic maps and tiny redacted reproductions with clear provenance. Valve campaign VMFs/BSPs and game assets are never committed.

## Release gate

A release requires:

- supported Python matrix passing;
- lint and type checks passing;
- schema compatibility tests;
- lossless corpus passing;
- every enabled transformation's golden/compiler/runtime suite passing;
- documentation/source ledger current;
- no unqualified compiler fingerprint accepted by default.
