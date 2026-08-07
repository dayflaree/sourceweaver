# BSP decompile import workflow

Source Weaver is VMF-first. BSP files are compiled game artifacts, while Source Weaver's parser, cleaner, previewer, and stitcher operate on editable VMF documents. BSP import therefore uses external tools to create VMFs before normal Source Weaver processing.

## Recommendation

Use BSPSource or another trusted external decompiler to generate a VMF, then import that VMF into Source Weaver.

BSPSource is the most viable current option because it is an actively maintained Source-engine BSP-to-VMF decompiler. Its project README describes it as a Java Source engine map decompiler that converts `.bsp` maps back to `.vmf` files for Hammer. BSPSource 1.4.8 was checked on 2026-08-06; its CLI accepts `-o <path>` to choose the VMF output path.

Source Weaver does not bundle BSPSource, VMEX, game BSPs, or decompiled content. The current implementation is first-class user-selected BSPSource execution plus a generic wrapper escape hatch. Managed downloads or bundled binaries remain deferred until dependency redistribution, checksums, update policy, and support expectations are reviewed for a release.

## CLI decompiler runner

Use `sourceweaver bsp-import` when you want Source Weaver to run a user-selected decompiler, capture the log, and validate the generated VMF. A BSPSource launcher from the Linux/Windows bundle no longer needs a wrapper script:

```bash
sourceweaver bsp-import map.bsp \
  --bspsource /path/to/bspsrc.sh \
  --output decompiled_map.vmf \
  --log decompile.log \
  --timeout-seconds 900 \
  --report bsp-import-report.json \
  --json
```

The BSPSource launcher command shape is:

```text
bspsrc [--tool-arg values...] -o <out.vmf> <input.bsp>
```

For jar-only BSPSource distributions, provide the jar and optionally the Java executable:

```bash
sourceweaver bsp-import map.bsp \
  --bspsource-jar /path/to/bspsrc.jar \
  --java /path/to/java \
  --output decompiled_map.vmf \
  --json
```

The jar command shape is:

```text
java -jar <bspsrc.jar> [--tool-arg values...] -o <out.vmf> <input.bsp>
```

Jar mode is also the safest option when a launcher script mishandles quoted paths. Some generated shell launchers forward arguments with unquoted `$*`, which can split BSP paths such as `Half-Life 2/hl2/maps/example.bsp`. Source Weaver preserves arguments when launching Java directly with `--bspsource-jar` and `--java`.

`--tool-arg` forwards one argument at a time before `-o`. Use it for BSPSource options such as `--unpack_embedded`, `--no_smart_unpack`, `--appid`, or `--format`.

The generic wrapper escape hatch remains available for unusual decompilers or argument orders:

```text
<wrapper> [--tool-arg values...] <input.bsp> <output.vmf>
```

The JSON report includes tool kind, tool path, BSPSource version probe when available, command arguments, input BSP, output VMF, exit code, log path, warning/error counts, entity count, classname count, and VMF integrity status. External decompiler runs default to a 900-second timeout; override with `--timeout-seconds` for slower maps or short failure tests.

## Desktop decompile/import workflow

The desktop app provides **Decompile BSP...** and a **BSP decompile import** panel. Select a `.bsp`, choose exactly one user-provided decompiler mode, and choose output/log/report paths:

- BSPSource launcher, using `bspsrc -o <out.vmf> <input.bsp>`;
- BSPSource jar, using `java -jar <bspsrc.jar> -o <out.vmf> <input.bsp>`;
- generic wrapper escape hatch for unusual tools or argument orders.

The desktop runner delegates to the CLI `bsp-import` workflow in a background worker. Successful output is parsed, integrity-checked, imported into the selected VMF list, and tagged as BSP-derived. The UI shows the command, JSON report, stdout/stderr tails, and decompile-quality warnings. Set `SOURCEWEAVER_CLI=/path/to/sourceweaver` before launching the desktop app if the CLI executable is not next to the desktop executable.

The desktop app also keeps **Add BSP-derived VMF...** for VMFs decompiled outside Source Weaver. Source Weaver adds those VMFs as normal VMF inputs while marking them as BSP-derived in the map list.

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

BSPSource licensing was checked live on 2026-08-06. The upstream repo contains `LICENSE.md` with Unlicense/public-domain text for BSPSource itself and notes Apache-2.0 dependencies for Log4j 2, Apache Commons Compress, picocli, FlatLaf, and jSystemThemeDetector plus BSD-3-Clause MigLayout. GitHub repository metadata still reports `NOASSERTION`/Other, so Source Weaver records the finding but does not bundle BSPSource in this slice.

## Future work

Future improvements can add managed BSPSource download with checksum/provenance review, richer decompile-warning parsers, known BSPSource argument presets, or legally committable tiny BSP-derived VMF fixtures. The VMF-first boundary should remain unchanged.

## Sources checked

- BSPSource upstream project: https://github.com/ata4/bspsrc
- BSPSource `LICENSE.md`: https://github.com/ata4/bspsrc/blob/master/LICENSE.md
- BSPSource v1.4.8 release: https://github.com/ata4/bspsrc/releases/tag/v1.4.8
- Valve Developer Union BSPSource page: https://valvedev.info/tools/bspsource/


## Managed BSPSource helper

Source Weaver keeps user-selected BSPSource launcher, jar, and wrapper paths as supported alternatives. For users who want a pinned upstream helper, `sourceweaver bspsource` provides:

- `manifest --json` for the pinned BSPSource version, asset URLs, sizes, and SHA-256 digests;
- `policy --json` for the licensing/provenance/cache/update policy;
- `cache-path` to show where a managed asset would be cached;
- `verify` to check a local BSPSource ZIP against a pinned asset or explicit SHA-256;
- `download` to perform a user-accepted, checksum-verified cache download.

Source Weaver does not bundle BSPSource or automatically adopt latest upstream releases. See `docs/bspsource-managed-download.md` for the research result and policy details.
