# Displacement translation notes

Source Weaver translates VMF displacement surfaces conservatively during landmark-aligned merges.

## Translated fields

The merge path translates the enclosing side `plane` points for every brush side, including sides that contain a `dispinfo` block. It also translates `dispinfo` `startposition` values because Source stores that absolute point in VMF displacement data.

Supported `startposition` formats:

- `[x y z]`, the common VMF displacement form
- `(x y z)`, accepted defensively for malformed or hand-authored fixtures
- `x y z`, accepted defensively

Example:

```vmf
"startposition" "[0 0 128]"
```

translated by offset `32 -16 8` becomes:

```vmf
"startposition" "[32 -16 136]"
```

## Fields intentionally left unchanged

Displacement normals, distances, alphas, triangle tags, allowed verts, and relative offset arrays are not absolute world-origin fields in the same way as `startposition`. Source Weaver preserves them as-authored while moving the brush side plane and `startposition` together.

## Validation coverage

Regression tests cover displacement-containing fixtures with:

- side plane translation
- square-bracket `startposition` translation
- parenthesized and bare-vector fallback parsing
- golden merged VMF output

## Remaining real-map validation

This Linux development environment does not include Hammer or Source SDK tooling. The portable `sourceweaver validate` command and VBSP log parser can validate generated VMFs and captured compile logs. When real displacement-heavy HL2 or Black Mesa maps are available, run:

```bash
sourceweaver validate stitched.vmf --compile-log vbsp.log --json
```

and compare the resulting JSON report against the generated VMF. Any game-specific displacement edge case found from real maps should become a focused fixture before changing translation behavior.
