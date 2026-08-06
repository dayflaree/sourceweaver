# BSP decompile import workflow

Source Weaver is VMF-first. BSP files are compiled game artifacts, while Source Weaver's parser, cleaner, previewer, and stitcher operate on editable VMF documents. BSP import should therefore remain an external pre-processing workflow unless a future ticket adds an explicit user-provided decompiler wrapper.

## Recommendation

Use BSPSource externally, then import the generated VMF into Source Weaver.

BSPSource is the most viable current option because it is an actively maintained Source-engine BSP-to-VMF decompiler. Its project README describes it as a Java Source engine map decompiler that converts `.bsp` maps back to `.vmf` files for Hammer. The Valve Developer Union tool page also describes BSPSource as Java-based, graphical, based on VMEX, and able to decompile Source BSPs into editable VMFs.

Source Weaver should not bundle BSPSource, VMEX, or game BSP assets. If integration is added later, it should be a thin wrapper around a user-provided executable/JAR path.

## External workflow

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

BSP decompilation is approximate. The Valve Developer Union page notes that decompiling may not produce a perfect VMF and that keyvalues, materials, variables, solids, instances, and areaportals may differ or break. Source Weaver should treat BSP-derived VMFs as untrusted inputs and rely on existing parse, integrity, preview, and compile-report workflows to expose problems.

Expected limitations include:

- brush geometry can be simplified, split, or invalid
- areaportals and instances can be broken
- material axes or texture information can differ from the authoring VMF
- entity keyvalues can be incomplete or modified by compile/decompile round trips
- overlays and side references can be fragile
- original editor metadata, visgroups, cameras, and cordons are often missing or reconstructed

## Legal and distribution constraints

Source Weaver should not ship game BSPs, decompiled maps, or third-party decompilers. Users are responsible for only decompiling maps they are legally allowed to inspect or modify. Decompilation can implicate game EULAs, mod licenses, server/community map licenses, and asset copyrights. Documentation and any future wrapper should state that BSP import is for legitimate modding, recovery, interoperability, or user-owned workflows.

## Future implementation tickets

Implementation work was split into follow-up issues:

- #74 Add optional user-configured BSPSource wrapper
- #75 Add desktop BSP-derived VMF import wizard

Future work should include decompile log capture, warning parsing, pre-import `inspect`/`validate`, and legally committable tiny fixture VMFs when available.

## Sources checked

- BSPSource upstream project: https://github.com/ata4/bspsrc
- Valve Developer Union BSPSource page: https://valvedev.info/tools/bspsource/
