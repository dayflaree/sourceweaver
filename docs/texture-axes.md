# Texture-axis translation

Source Weaver translates VMF brush geometry by moving side `plane` points. Texture axes need matching offset updates so textures stay visually locked to the moved brush instead of sliding across it.

## VMF texture-axis format

Source VMF sides commonly store axes like this:

```vmf
"uaxis" "[1 0 0 0] 0.25"
"vaxis" "[0 -1 0 0] 0.25"
```

The bracket contains the axis vector and shift value. The final number is the texture scale.

## Translation rule

For each translated brush side:

```text
new_shift = old_shift - dot(axis_vector, translation_offset)
```

The axis vector and scale are preserved. This keeps the texture coordinate at the moved surface point equal to the coordinate before movement, matching texture-lock behavior for whole-map translations.

Examples:

```text
uaxis [1 0 0 16] 0.25, offset 32 0 0  -> [1 0 0 -16] 0.25
vaxis [0 -1 0 8] 0.5, offset 0 16 0   -> [0 -1 0 24] 0.5
```

## Validation coverage

Regression tests cover:

- direct `uaxis` and `vaxis` offset adjustment
- positive and negative axes
- side-block translation alongside planes, origins, and displacement `startposition`
- golden merged output snapshots

## Real-map validation

This Linux development environment does not include Hammer or Source SDK tooling. The expected texture-lock behavior is implemented from the VMF axis equation and guarded by tests. When real HL2/Black Mesa textured fixtures or compile screenshots/logs are available, capture them with the portable validation workflow:

```bash
sourceweaver validate stitched.vmf --compile-log vbsp.log --json
```

If a game-specific material projection case differs, add a small VMF fixture that captures the observed `uaxis`/`vaxis` values before changing the translation rule.
