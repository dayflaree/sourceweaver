# Local GMod Tools++ observation — 2026-07-26

## Environment

A local Garry's Mod installation contained four 64-bit Windows PE executables under `bin/win64`:

- `vbspplusplus.exe`
- `vvisplusplus.exe`
- `vradplusplus.exe`
- `bspzipplusplus.exe`

Exact hashes and sizes are stored in `../compiler-fingerprints/gmod-toolsplusplus-local-2026-07-26.json`.

## Static observations

`file` identified the tools as PE32+ executables. Printable-string inspection found:

- VBSP++ messages for brush/plane/side limits, areaportal leaks, areas, portals, `-embed`, target BSP formats, and `-threads`.
- VBSP++ warnings that `func_viscluster` is deprecated and unnecessary with VVIS++.
- VVIS++ portal-cluster, portal-flow, overflow, and threading messages.
- VRAD++ Source branch/BSP parsing and threading messages.
- BSPZIP++ `-repack`, compression, and threading messages.

These observations confirm that the files contain the expected tool functionality. Printable strings do not prove runtime behavior or supported values.

## Invocation attempt

A help invocation through an isolated GE-Proton prefix initialized the prefix but exited with code 1 without compiler help output. The attempt therefore did not qualify execution. Windows-native or correctly configured Wine/Proton fixture compiles remain required.

## Design consequence

SourceWeaver discovers and hashes these executables. It treats the hashes as unqualified until the compiler fixture suite and GMod runtime suite pass.
