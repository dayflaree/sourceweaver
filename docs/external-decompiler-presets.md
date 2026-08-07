# External BSP decompiler presets and VMEX research

Source Weaver's supported BSP import path remains BSPSource through user-provided launchers, jars, or generic wrappers. Alternative decompilers are handled conservatively: Source Weaver can document wrapper shapes and report integration metadata, but it does not bundle third-party binaries or claim real decompiler validation unless the tool was actually run.

## CLI registry

```bash
sourceweaver external-decompiler-presets --json
```

The report lists:

- `bspsource-supported` — the normal supported local-path/managed-manifest BSPSource integration path;
- `vmex-legacy-wrapper` — a documentation-only VMEX wrapper example;
- `unknown-wrapper-template` — generic wrapper escape hatch for user-provided tools.

Each entry reports tool status, maintenance notes, license summary, source URL, checked date, command shape, Source Weaver workflow, wrapper example, caveats, bundle policy, and whether real tool validation was performed.

## VMEX research result

Research source checked on 2026-08-06:

- Valve Developer Union VMEX page: `https://valvedev.info/tools/vmex/`.
- The page marks VMEX **obsolete** and says it has largely been supplanted by BSPSource.
- The page says VMEX does not support post-Orange Box Source games.
- The page says VMEX is no longer in active development and is archived for historical reasons.
- The page documents GUI use and command-line use as `vmex [path to map.bsp]`.
- The page says decompiled map names have `_d` appended.
- The page warns decompiled VMFs may have broken complex solids, rounding/misshapen geometry, missing editor-specific data such as visgroups, and unusable areaportals without post-decompile fixing.
- The page says VDU guides are CC BY-SA 4.0, but also says tools belong to their respective coders. No VMEX binary redistribution license was verified during this issue.

Decision:

- Do not bundle VMEX.
- Do not implement managed VMEX download.
- Do not mark VMEX as a supported/validated decompiler.
- Provide only documentation and a generic-wrapper example for users who already have VMEX locally.

## VMEX wrapper example

`examples/wrappers/vmex-wrapper.sh` demonstrates how Source Weaver's generic wrapper contract can adapt VMEX's documented `_d` output behavior:

```bash
sourceweaver bsp-import \
  --tool examples/wrappers/vmex-wrapper.sh \
  input.bsp \
  --output output.vmf
```

Source Weaver invokes generic wrappers as:

```text
<wrapper> [tool-args] <input.bsp> <out.vmf>
```

The example wrapper:

1. reads the requested input BSP and output VMF path;
2. runs `${VMEX_BIN:-vmex}` with any extra wrapper args and the input BSP;
3. expects VMEX to create `<input_stem>_d.vmf` next to the input BSP;
4. moves that VMF to the Source Weaver-requested output path.

This wrapper was not validated with a real VMEX binary in CI. It is an example based on documented VMEX command behavior only.

## Generic wrapper template

Unknown or proprietary decompilers should be integrated through the existing generic wrapper mode:

```bash
sourceweaver bsp-import \
  --tool /path/to/wrapper.sh \
  --tool-arg custom \
  input.bsp \
  --output output.vmf \
  --json
```

The wrapper must write the requested output VMF path. Source Weaver then performs the same native VMF parsing, integrity validation, decompile-quality log parsing, and BSP-derived review labeling.

## Validation boundary

No real VMEX, alternative decompiler, proprietary BSP, game content, SDK, Hammer, or BSPSource run was performed for this issue. The implementation validates only Source Weaver's registry JSON, documentation, and wrapper file presence. Real external decompiler validation requires a real tool run with captured command/version/log/report evidence and redistribution review for any committed fixtures.
