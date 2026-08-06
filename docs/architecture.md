# Source Weaver architecture

## Layers

### `sourceweaver-core`

The core crate owns VMF behavior:

- VMF tokenization and parsing
- VMF serialization
- entity and brush inspection
- landmark origin lookup
- brush and entity translation
- merge operations
- deletion/prune operations

The core crate should remain dependency-light and UI-agnostic.

### `sourceweaver-cli`

The CLI is the first executable interface. It is used for development validation, scripting, and regression tests.

Current commands:

- `inspect`
- `list-types`
- `prune`
- `merge`

### Future desktop app

The desktop UI should sit above `sourceweaver-core`. Recommended UI candidates remain open, but the UI must expose:

- file picker for VMF selection
- base map selector
- landmark selector
- entity table
- brush role filters
- classname and targetname filters
- bulk selection and deletion preview
- merge report
- output path picker

## VMF model

Source Weaver currently represents VMF as a generic ordered KeyValues tree:

```text
Document
  Node::Block { name, body }
  Node::Property { key, value }
```

This preserves unknown Hammer and game-specific data because the parser does not discard unrecognized blocks or keys.

## Merge model

The first selected VMF is the base document. For each additional VMF:

1. Parse VMF.
2. Find requested `info_landmark` targetname.
3. Compute translation offset against the base landmark.
4. Translate incoming entity `origin` values.
5. Translate incoming brush `plane` values.
6. Translate displacement `startposition` values when present.
7. Renumber incoming `id` keys.
8. Append incoming world solids into the base `world` block.
9. Append incoming top-level entities after existing base nodes.

## Deletion model

Deletion is criteria-based. The UI should build a `DeletionCriteria` object from selected rows or filters, then call the same prune function as the CLI.

Brush-role deletion removes matching world solids and whole brush entities that match the selected role.

## Known technical risks

- Displacement data may need more coordinate translation beyond `startposition`.
- Texture lock behavior may require updating texture-axis offsets after brush translation.
- Some maps contain nested/hidden groups that need more nuanced merge behavior.
- VMF instance handling may require expanding or preserving `func_instance` workflows.
- Hammer compile limits may be reached when many campaign maps are merged.
