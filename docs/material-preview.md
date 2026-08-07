# Material-aware preview

Source Weaver's desktop preview can color reconstructed brush faces by VMF `side` material names. This is a user-root-only preview feature: Source Weaver does not bundle game content, custom materials, SDK assets, BSP-extracted files, VTF decoders, or any Valve game files.

## Safe asset-loading strategy

The desktop UI has a **Material-aware faces** toggle and **Material roots** field in the preview controls. Roots are comma-separated folders chosen by the user. They can point at a `materials/` directory or a game/mod/content folder that contains a `materials/` subtree.

When **Scan material roots** is clicked, Source Weaver recursively scans the selected roots for:

- `.vmt` material definition files;
- `.vtf` Source texture files;
- optional portable preview sidecars such as `.png`, `.jpg`, `.jpeg`, and `.tga`.

Source Weaver normalizes material names by lowercasing, replacing `\` with `/`, stripping a leading `materials/`, and removing the extension. It also reads simple VMT `$basetexture` references so a material file can make its base texture name count as available.

No scanned file is redistributed. The scan only builds an in-memory index for the current desktop session.

## Rendering behavior

The core preview extracts normalized material names from VMF `side` blocks and stores both per-solid material lists and per-reconstructed-face material names. The desktop preview uses this metadata when reconstructed faces are drawn.

Face colors follow this policy:

- **Available materials** found in a scanned root receive a stable, brighter material-specific color.
- **Missing materials** receive a dimmer stable material-specific fallback color so missing content remains visible.
- **Tool materials** such as trigger, clip, skybox, nodraw, hint, and skip receive fixed semantic colors, independent of scanned roots.
- Deletion highlighting and selection strokes continue to override or decorate the material colors.

This implementation is material-aware rather than a full Hammer textured viewport. It does not decode VTF pixels or project texture UVs. It intentionally avoids loading game content by default and avoids claiming exact in-game material appearance.

## Performance tradeoffs

Material scanning is manual so large game/content trees are not scanned repeatedly while editing. The scanner has a recursion depth limit and reports unreadable folders as warnings in the preview panel. Rendering material colors uses the existing reconstructed face geometry path:

- **Fast** detail mode skips reconstructed faces, so material-aware face colors are not shown.
- **Auto** shows reconstructed faces only below the existing solid-count threshold.
- **Full** forces reconstructed faces and may be slower on very large maps.
- Selected solids draw reconstructed faces regardless of detail mode, so material color can be inspected locally.

## Fixtures and tests

- Core preview tests verify VMF side material extraction and face-material alignment.
- Desktop unit tests verify user-root scanning for VMT/VTF/portable preview files and fixed/available/missing material colors.

## Validation boundary

Material-aware preview is a Source Weaver desktop visualization feature. It does not run Hammer, Hammer++, VBSP, VVIS, VRAD, BSPZIP, a game runtime, a game SDK, or any external material tool. It does not prove compile or runtime material availability.
