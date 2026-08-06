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

## Camera controls

Select **3D iso** in the preview toolbar. The 3D camera controls appear below the view selector:

- **Yaw** rotates the view around the vertical axis.
- **Pitch** tilts the view.
- **Reset 3D camera** restores the default isometric camera.
- Mouse-wheel zoom and drag-pan work as they do in 2D.
- Preview click selection works against the projected entities/solid bounds.

## Design scope

This is a geometry-first validation viewport. It is not a textured Hammer clone yet. Material texture rendering and game-specific model/entity icons remain future work. The current goal is to visually validate stitched geometry, landmark alignment, tool brushes, and entity origins without leaving Source Weaver.

## Fallbacks

Malformed or open brush solids can still fall back to bounds rendering. The 2D top/front/side views remain available and faster for precise orthographic layout checks.
