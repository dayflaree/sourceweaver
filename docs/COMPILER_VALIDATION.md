# Compiler validation

## Exact-toolchain rule

Every verdict is tied to exact executable hashes. Filenames and version labels are insufficient because community and game compilers change independently.

A compiler profile records:

- absolute discovery path locally;
- basename in portable reports;
- SHA-256 and size;
- file metadata/version when available;
- supported invocation mode;
- target `gameinfo.txt` and search paths;
- command-line flags;
- environment and working directory;
- qualification status.

Compiler binaries are never committed or redistributed.

## Qualification

A newly observed executable hash is quarantined until it passes:

- minimal sealed map;
- intentional world leak;
- valid areaportal;
- areaportal bypass leak;
- hint fixture;
- displacement fixture;
- instance fixture;
- packed-resource fixture;
- branch-specific BSP/static-prop fixture;
- deterministic repeated compile comparison.

VVIS++’s same-result claim is verified through controlled differential BSP/PVS comparisons for every adopted hash where a stock comparator is available.

## Compile tiers

### Structural

- VBSP with leak testing.
- No VVIS/VRAD unless needed for the candidate.
- Used for quick geometry rejection.

### Visibility fast

- VBSP plus fast visibility mode when supported.
- Used only to rank candidates.
- Cannot produce a final acceptance verdict for visibility changes.

### Visibility full

- VBSP plus full VVIS/VVIS++.
- Required for areaportal and hint acceptance.

### Lighting validation

- VRAD/VRAD++ with the profile's representative quality mode.
- Required when geometry, materials, lightmaps, props, or lighting controllers changed.

### Release

- Exact production flags.
- Asset packing and BSP repack/compression if configured.
- Final runtime suite.

## Process control

- Use argument arrays, never shell-concatenated user input.
- Set explicit working directories.
- sanitize inherited environment variables;
- capture stdout and stderr losslessly;
- enforce per-stage timeout and output-size limits;
- kill the complete process tree on failure;
- preserve partial artifacts for diagnosis;
- never overwrite a known-good BSP.

## Implemented support envelope

Current code supports compiler discovery, executable fingerprinting, executable-format detection, host compatibility classification, and compiler-run preflight for required compile stages (`vbsp`, `vvis`, `vrad`). The preflight reports:

- missing required compiler executables;
- native executable readiness;
- Windows PE compilers on Linux requiring a compatibility runner;
- compatibility-runner discovery through `wine64`/`wine`;
- unsupported executable formats and unknown hosts.

This preflight does not invoke compilers, parse logs, inspect BSP/PRT artifacts, qualify hashes, or produce acceptance verdicts. A discovered compiler set can still be blocked if the host lacks a required runner.

Current code also builds deterministic compile invocation plans when preflight is ready, the source VMF exists, and the map name is non-empty. Plans define argument arrays for `vbsp`, `vvis`, and `vrad`, stage work directories, stdout/stderr log paths, and the expected BSP artifact path. Windows PE tools on Linux are prefixed with the selected compatibility runner from preflight. Invocation plans are read-only and do not spawn processes.

Current code also parses compiler stdout/stderr into normalized log messages. The parser records blocking leaks, limits, fatal errors, unknown error-like output, and non-blocking portal/area statistics. Log reports remain evidence only: they do not inspect BSP/PRT artifacts or produce an acceptance verdict by themselves.

Current code also performs minimal BSP/PRT artifact inspection. BSP inspection verifies artifact presence, `VBSP` magic, complete Source header size, BSP version, and per-lump lengths. PRT inspection verifies artifact presence and extracts top-level leaf and portal counts from ASCII PRT files. Artifact reports remain evidence only and do not yet compare baseline/candidate outputs or qualify tool hashes.

## Log parser

Messages are classified by exact compiler fingerprint and normalized code:

- fatal error;
- world leak;
- areaportal leak;
- limit overflow;
- malformed brush/face/displacement;
- missing asset/material/instance;
- warning with acceptance policy;
- informational statistic;
- unknown output.

Unknown compiler output is retained and raises the review level. New error-like lines cannot be silently ignored.

## Artifact inspection

Do not rely solely on process exit code. Inspect:

- expected BSP and PRT presence;
- BSP header/version;
- lump bounds and parseability;
- entity and static-prop data;
- planes, nodes, leaves, faces, clusters, visibility, areas, areaportals, overlays;
- embedded pak contents;
- leak line files;
- output timestamps and hashes.

## Baseline parity

Every candidate is compared with a baseline compiled in the same environment. A toolchain update invalidates cached acceptance results.

## Determinism

Compile the same fixture at least twice during qualification. Classify differences by lump. Known nondeterministic data may be masked only through a documented, fingerprint-specific rule. Geometry, visibility, entity, and relevant lighting differences must remain explainable.
