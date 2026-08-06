# Editor metadata merge policy

Source Weaver keeps merge output Hammer-friendly by treating top-level editor metadata conservatively.

## Preserved from the base VMF

The base VMF remains the root document. Source Weaver preserves base top-level editor and document metadata as-authored, including sections such as:

- `versioninfo`
- `viewsettings`
- `visgroups`
- `cameras`
- `cordons`
- other unknown top-level metadata blocks

This keeps the output anchored to the base map's editor settings and avoids conflicting global editor sections from multiple maps.

## Ignored from incoming VMFs

Incoming top-level metadata sections are intentionally not merged. Source Weaver imports only incoming world solids and top-level `entity` blocks, after translation, ID renumbering, and known ID-reference remapping.

Ignored incoming top-level sections include:

- `versioninfo`
- `viewsettings`
- `visgroups`
- `cameras`
- `cordons`
- unknown top-level metadata blocks

This avoids duplicate global editor settings and mismatched visgroup/camera/cordon state in the merged VMF.

## Preserved inside appended objects

Nested editor metadata inside appended world solids or entities is preserved with that object. Examples include nested `editor` blocks that carry colors, group state, or object-level editor fields. Known ID references inside those nested blocks are remapped when the referenced old ID maps uniquely to a new ID.

## Unsupported metadata

Unknown top-level metadata from incoming VMFs is ignored intentionally. Unknown nested metadata on imported objects is preserved because it travels with the object it describes.

If a future real-map fixture proves a top-level metadata section must be merged, add a focused fixture and merge policy test before changing this behavior.
