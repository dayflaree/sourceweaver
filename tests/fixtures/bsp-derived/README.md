# Synthetic BSP-derived fixture set

This directory contains a legally redistributable fixture set for Source Weaver BSP import regression tests.

## Files

- `tiny_synthetic_header.bsp`: Source Weaver-authored binary fixture containing only a minimal Source BSP-style header (`VBSP`, version 20, zeroed lump descriptors, map revision 0). It is not a playable map and contains no proprietary content.
- `tiny_synthetic_generated.vmf`: Source Weaver-authored expected VMF used by CI fake-wrapper tests.
- `manifest.json`: machine-readable provenance, license, command boundary, redaction, and checksum metadata.

## Legal/provenance boundary

These files were created from scratch for Source Weaver tests and contain no Valve, game, mod, custom-map, or proprietary assets. The fixture set is intended to be redistributable under CC0-1.0.

No real BSPSource run was used to create these files. The CI regression test uses a fake wrapper that copies the committed VMF when invoked with the committed BSP input. This exercises Source Weaver's import/report/validation path without requiring proprietary BSPs or external decompiler tools.

Real BSPSource validation remains outside the repository unless the BSP input, generated VMF, tool version, command line, and redistribution license are all verified and recorded.
