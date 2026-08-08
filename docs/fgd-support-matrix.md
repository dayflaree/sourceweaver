# FGD parser support matrix

Source Weaver uses FGD files as optional entity metadata. It does not implement the full Hammer or TrenchBroom FGD language. The parser is intentionally conservative: supported class and property metadata is loaded for table labels/tooltips, while unsupported language features are skipped without turning them into fake entity classes.

## Current supported syntax

| Syntax | Status | Behavior |
| --- | --- | --- |
| `@PointClass ... = classname : "Description"` | supported | Adds class metadata with point category and optional description. |
| `@SolidClass ... = classname : "Description"` | supported | Adds class metadata with brush category and optional description. |
| `@NPCClass ... = classname : "Description"` | supported | Adds class metadata with NPC category and optional description. |
| class helpers such as `base(...)`, `iconsprite(...)`, `size(...)`, `model(...)` before `=` | metadata ignored | Class metadata still loads, but helper settings are not evaluated. |
| property line `key(type) : "Label" : default : "Description"` | supported | Adds property key, type, label, default value, and description. |
| property line `key(type) = "Label" : default : "Description"` | supported | Same property metadata path; useful for Source-style variants. |
| `choices` blocks | supported | Choice value, label, and optional description are loaded. |
| `flags` blocks | supported | Flag value, label, and optional description are loaded. |
| comments beginning with `//` | skipped | Safe no-op. |
| unknown valid property types | supported as metadata strings | Source Weaver records the type name but does not attach Hammer semantics. |
| invalid classnames or property keys | skipped | Prevents malformed or unsupported declarations from becoming metadata. |

## Unsupported syntax and boundaries

| Syntax or feature | Status | Behavior |
| --- | --- | --- |
| `@include` | unsupported | Skipped. Users may load multiple FGD files explicitly. Source Weaver does not resolve include paths. |
| `@BaseClass` inheritance definitions | unsupported | Skipped. Source Weaver does not merge inherited properties into derived classes. |
| recursive or multiple inheritance | unsupported | `base(...)` is recorded only as ignored helper text. |
| input/output declarations | unsupported | Lines such as `input Fire(void)` and `output OnTrigger(void)` have invalid keys for Source Weaver metadata and are skipped. |
| helper metadata evaluation | unsupported | `size`, `model`, `iconsprite`, sprites, colors, studio previews, and editor-display helpers are not evaluated. |
| expressions, conditionals, placeholders, or game-specific editor extensions | unsupported | Skipped unless they look like a simple supported property line. |
| complex nested blocks other than simple choices/flags | unsupported | Unsupported nested content is skipped best-effort. |
| full Hammer FGD parity | not claimed | Source Weaver only loads selected metadata needed for entity tables and tooltips. |

## Workflow decision

The supported subset is enough for current Source Weaver workflows:

- friendly class descriptions in entity tables;
- optional override of built-in/inferred class metadata;
- property labels, defaults, descriptions, choices, and flags for tooltips/search;
- graceful handling of unknown game-specific entities.

Full inheritance and editor-helper evaluation are not needed for current merge, validation, deletion, preview, or release workflows. They should be added only after a fixture demonstrates the exact syntax and a UI/CLI use case needs the extra metadata.

## Validation fixture

`tests/fixtures/fgd_support_matrix.fgd` is Source Weaver-authored and redistributable. It includes:

- an `@include` line;
- an unsupported `@BaseClass` with inherited property text;
- `@PointClass`, `@SolidClass`, and `@NPCClass` declarations;
- choices and flags blocks;
- invalid input/output/key lines that must be skipped.

The unit test `parses_supported_fgd_matrix_and_skips_unsupported_language` verifies the support matrix and confirms `@BaseClass = Targetname` is not emitted as fake entity metadata.

## Release wording

Allowed:

```text
Source Weaver loads a lightweight FGD metadata subset for class descriptions and selected property metadata, including choices and flags.
```

Required limitation:

```text
Source Weaver does not implement full Hammer FGD parity, include resolution, BaseClass inheritance merging, helper metadata evaluation, expressions, conditionals, or game-specific editor extensions.
```

## External reference checked

- The Level Design Book FGD file-format page, checked 2026-08-08: https://book.leveldesignbook.com/appendix/resources/formats/fgd . It describes FGD as an editing-aid format rather than engine functionality, notes there is no formal FGD specification standard, and describes core point/solid/base entity types, `base(...)` inheritance, property definitions, `choices`, `flags`, model helpers, placeholders, and conditional/editor-specific syntax. This supports Source Weaver's documented lightweight subset instead of a full-parity claim.
