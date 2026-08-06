# BSP decompile import workflow

Source Weaver is VMF-first. BSP files are compiled game artifacts, while Source Weaver's parser, cleaner, previewer, and stitcher operate on editable VMF documents. BSP import therefore uses external tools to create VMFs before normal Source Weaver processing.

## Recommendation

Use BSPSource or another trusted external decompiler to generate a VMF, then import that VMF into Source Weaver.

BSPSource is the most viable current option because it is an actively maintained Source-engine BSP-to-VMF decompiler. Its project README describes it as a Java Source engine map decompiler that converts `.bsp` maps back to `.vmf` files for Hammer. The Valve Developer Union tool page also describes BSPSource as Java-based, graphical, based on VMEX, and able to decompile Source BSPs into editable VMFs.

Source Weaver does not bundle BSPSource, VMEX, game BSPs, or decompiled content. Integration is a thin wrapper around a user-provided executable or wrapper script.

## CLI wrapper

Use `sourceweaver bsp-import` when you want Source Weaver to run an external decompiler, capture the log, and validate the generated VMF:

```bash
sourceweaver bsp-import map.bsp \
  --tool /path/to/bspsource-or-wrapper \
  --output decompiled_map.vmf \
  --log decompile.log \
  --timeout-seconds 900 \
  --report bsp-import-report.json \
  --json
```

The generic command shape is:

```text
<decompiler> [--tool-arg values...] <input.bsp> <output.vmf>
```

If BSPSource or another tool needs a different command-line shape, create a small wrapper script and pass that script as `--tool`. The JSON report includes the tool path, input BSP, output VMF, exit code, log path, warning/error counts, entity count, classname count, and VMF integrity status. External decompiler runs default to a 900-second timeout; override with `--timeout-seconds` for slower maps or short failure tests.

## Desktop workflow

The desktop app provides **Add BSP-derived VMF...**. Use it after an external decompiler has produced a VMF. Source Weaver adds the generated VMF as a normal VMF input while marking it as BSP-derived in the map list. The UI warns users to review decompile limitations, parse/integrity warnings, broken solids, areaportals, materials, overlays, and missing editor metadata before merging.

## Manual external workflow

1. Install BSPSource from the upstream project or trusted release source.
2. Decompile the `.bsp` into a `.vmf` with BSPSource.
3. Open the generated VMF in Hammer/Hammer++ when available and inspect obvious decompile defects.
4. Run Source Weaver on the generated VMF:

```bash
sourceweaver inspect decompiled_map.vmf
sourceweaver validate decompiled_map.vmf --json
```

5. If stitching with another map, use the normal VMF workflow:

```bash
sourceweaver merge -o stitched.vmf --landmark map_transition base.vmf decompiled_map.vmf
sourceweaver validate stitched.vmf --json
```

6. When Source compile tools are available, run the optional compile pipeline:

```bash
sourceweaver compile stitched.vmf --profile hl2-tools.toml --steps vbsp,vvis,vrad --json
```

## Known decompile limitations

BSP decompilation is approximate. The Valve Developer Union page notes that decompiling may not produce a perfect VMF and that keyvalues, materials, variables, solids, instances, and areaportals may differ or break. Source Weaver should treat BSP-derived VMFs as untrusted inputs and rely on parse, integrity, preview, and compile-report workflows to expose problems.

Expected limitations include:

- brush geometry can be simplified, split, or invalid
- areaportals and instances can be broken
- material axes or texture information can differ from the authoring VMF
- entity keyvalues can be incomplete or modified by compile/decompile round trips
- overlays and side references can be fragile
- original editor metadata, visgroups, cameras, and cordons are often missing or reconstructed

## Legal and distribution constraints

Source Weaver does not ship game BSPs, decompiled maps, or third-party decompilers. Users are responsible for only decompiling maps they are legally allowed to inspect or modify. Decompilation can implicate game EULAs, mod licenses, server/community map licenses, and asset copyrights. BSP import is for legitimate modding, recovery, interoperability, or user-owned workflows.

## Future work

Future improvements can add richer decompile-warning parsers, known BSPSource argument presets, or legally committable tiny BSP-derived VMF fixtures. The VMF-first boundary should remain unchanged.

## Sources checked

- BSPSource upstream project: https://github.com/ata4/bspsrc
- Valve Developer Union BSPSource page: https://valvedev.info/tools/bspsource/
