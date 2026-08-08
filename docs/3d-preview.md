# 3D preview viewport

Source Weaver includes a lightweight 3D isometric preview mode alongside the faster 2D top/front/side views.

## What it renders

The 3D preview uses the same `PreviewDocument` data as the 2D views:

- reconstructed convex brush face polygons
- role/source/deletion/selection coloring
- tool brushes such as triggers, clips, skybox, areaportals, occluders, and water
- entity origin markers
- `info_landmark` markers and labels
- merged-preview source colors and offset annotations

This keeps the 3D view consistent with preview/export behavior because it reuses the same parsed VMF and in-memory merge path.

## Preview detail and performance

Large BSPSource-decompiled VMFs can contain thousands of brushes and many more side triangles. Use the preview **Detail** selector to keep interaction responsive:

- **Fast boxes** draws projected brush bounds only.
- **Auto** keeps full detail for small maps, skips side-edge overlays on medium maps, and switches to boxes for large maps.
- **Full faces** draws reconstructed faces and side-edge overlays for inspection.

Use **Fast boxes** or **Auto** when previewing combined campaign maps or very large decompiled VMFs.

## Camera controls

Select **3D iso** in the preview toolbar. The 3D camera controls appear below the view selector:

- **Yaw** rotates the view around the vertical axis.
- **Pitch** tilts the view.
- **Reset 3D camera** restores the default isometric camera.
- Mouse-wheel zoom and drag-pan work as they do in 2D.
- Preview click selection works against the projected entities/solid bounds.

## Design scope

This is a geometry-first validation viewport. It is not a textured Hammer clone. Material-color rendering is available through `docs/material-preview.md`; decoded sidecar textures and optional VTF decoding are future work bounded by `docs/textured-preview-roadmap.md`. The current goal is to visually validate stitched geometry, landmark alignment, tool brushes, and entity origins without leaving Source Weaver.

## Fallbacks

Malformed or open brush solids can still fall back to bounds rendering. The 2D top/front/side views remain available and faster for precise orthographic layout checks.
