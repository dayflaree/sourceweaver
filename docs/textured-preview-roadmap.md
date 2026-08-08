# Textured preview and VTF decoding roadmap

Source Weaver's current preview is material-aware, not texture-decoded. It scans user-selected material roots for VMT/VTF/portable sidecar presence and colors faces deterministically by material name. It does not decode VTF pixels, sample texture coordinates, emulate Source shaders, or claim Hammer/runtime visual parity.

## Decision

VTF pixel decoding is **not in the current implementation scope**. The next supported stage should be implemented in two separate layers:

1. **Portable sidecar texture previews first.** Source Weaver may render user-provided `.png`, `.jpg`, `.jpeg`, or `.tga` sidecars from selected material roots after explicit user opt-in. These fixtures can be Source Weaver-authored and redistributable, which keeps tests and examples legally clean.
2. **Optional VTF decoder adapter later.** Native or helper-library VTF decoding should be added only after dependency licensing, security review, fixture generation, and format support boundaries are recorded. The adapter must be optional because users can already work with proprietary game material trees that Source Weaver must not redistribute.

This means preview release wording stays:

```text
Texture preview: material-aware colors only; no VTF pixel decoding or Hammer-equivalent textured viewport.
```

## Current supported preview inputs

| Input | Current behavior |
| --- | --- |
| VMF `side` material names | Extracted and attached to reconstructed face metadata. |
| VMT files in user-selected roots | Indexed as available materials; simple `$basetexture` references are indexed. |
| VTF files in user-selected roots | Indexed as texture-file presence only. Pixels are not decoded. |
| `.png`, `.jpg`, `.jpeg`, `.tga` sidecars | Indexed as portable preview texture presence only. Pixels are not rendered yet. |
| Tool materials | Rendered with fixed semantic colors. |
| Missing materials | Rendered with dim stable fallback colors. |

## Future sidecar-rendering scope

A sidecar-rendering implementation is acceptable when all of these are true:

- It is opt-in and local-session-only.
- It only reads user-selected roots.
- Tests use Source Weaver-authored generated images stored in fixtures or generated under `/tmp`.
- The UI offers a mode choice such as **Material colors** and **Sidecar textures**.
- Missing, unreadable, oversized, or unsupported images fall back to the existing material-color path.
- Texture rendering is documented as an approximation over reconstructed convex faces.

Initial sidecar tests should cover:

- generated 2×2 and 4×4 PNG fixtures;
- generated TGA fixture if the selected image loader supports it;
- missing sidecar fallback;
- unreadable/corrupt sidecar fallback;
- large-texture size cap behavior;
- material-color mode unchanged when texture mode is disabled.

## Optional VTF decoder support matrix

A future VTF decoder must publish a support matrix before it can be enabled by default. The first safe target should be narrow:

| VTF feature | First supported target | Boundary |
| --- | --- | --- |
| Version | 7.2 through 7.5 only after parser tests exist | Reject or color-fallback other versions. |
| Dimensions | 2D power-of-two textures within a configured pixel cap | Reject volume textures and very large textures until memory limits are proven. |
| Frames | first frame only | Animated textures remain unsupported until frame timing/UI is implemented. |
| Mipmaps | highest-resolution or explicitly selected mip level | Mipmap generation/streaming is not required. |
| Image formats | start with uncompressed RGBA8888, RGB888, BGR888, BGRA8888 and one compressed format only after tests | No broad "all VTF formats" claim. |
| Resources | high-res image only | Low-res thumbnails, CRC, LOD settings, extra flags, particle sheet data, and custom resources are ignored unless implemented. |
| Flags | metadata for warnings only | ENVMAP, SS_BUMP, NORMAL, RENDER_TARGET, PROCEDURAL, clamp/filtering, and sRGB flags do not imply shader/runtime parity. |
| Cubemaps | unsupported | Cubemap preview remains separate from material face texture preview. |
| VMT proxies/shaders | unsupported | Source shader graphs, `$envmap`, `$bumpmap`, `$detail`, `$selfillum`, `$translucent`, and proxies are not evaluated. |

The implementation should expose VTF decoding as **experimental** until fixtures cover every claimed version/format and the validation-claim guard allows only the exact supported wording.

## UI plan

Current UI:

- **Material-aware faces** checkbox.
- **Material roots** field.
- **Scan material roots** button.

Future UI when sidecar/VTF rendering exists:

- **Texture mode** selector:
  - `Material colors` — current behavior and default.
  - `Sidecar textures` — user-provided portable images only.
  - `VTF textures (experimental)` — enabled only when a decoder is compiled/configured and a support matrix exists.
- Warning banner when texture mode is approximate.
- Per-root scan summary including material count, VMT count, VTF count, sidecar count, decoded texture count, rejected texture count, and fallback count.
- Size-limit setting or hard cap with warnings to prevent unbounded memory use.

## Fidelity limits

Source Weaver preview must keep these limitations visible:

- No exact Hammer UV projection claim.
- No lightmap rendering.
- No cubemap/environment-map rendering.
- No shader/proxy evaluation.
- No animated texture playback.
- No normal/specular/detail/self-illum/translucency parity.
- No in-game material availability guarantee.
- No Hammer/Hammer++ viewport parity.
- No runtime screenshot parity.

Decoded texture previews, when implemented, will be a visual aid for identifying material placement. They will not be evidence of compile success, runtime correctness, or editor compatibility.

## Validation requirements before claims change

Before any release can claim decoded texture rendering, evidence must include:

```bash
cargo test --workspace
scripts/validate-material-preview-scope.sh /tmp/sourceweaver-material-preview-scope
python3 scripts/check-validation-claims.py --self-test
python3 scripts/check-validation-claims.py
```

The new decoded-texture tests must use generated or Source Weaver-authored fixtures. No proprietary VTF/VMT/material files may be committed. Real user material-root tests may be recorded in issue evidence only when paths are sanitized and artifacts stay outside the repository.

## External references checked

- Valve Developer Community VTF page was attempted on 2026-08-08 but returned an Anubis anti-scraping challenge instead of readable format content.
- `srctools.vtf` documentation, checked 2026-08-08: https://srctools.readthedocs.io/en/v2.3.13/modules/vtf.html . It documents VTF as Valve's texture format, supported versions `(7, 2)` through `(7, 5)`, mipmaps, resources, flags, frames, cube sides, and many image formats such as RGBA8888, RGB888, BGR888, BGRA8888, DXT1, DXT3, DXT5, ATI1N, and ATI2N. This supports a narrow, explicit support matrix rather than a broad VTF-parity claim.
