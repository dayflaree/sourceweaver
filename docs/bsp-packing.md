# BSP content packing workflow

Source Weaver can run an optional BSP packing step with a user-provided `bspzip`, BSPZIP++, or compatible external tool. Packing is separate from VMF editing, VMF merging, BSP decompiling, and VMF-to-BSP compiling.

Source Weaver does not bundle Valve tools, third-party packers, compiled BSPs, or custom assets. Users are responsible for owning or having permission to distribute every asset they pack. `docs/third-party-redistribution-policy.md` controls any future managed packer download or redistributable packing fixture.

## CLI workflow

Generate a BSPZIP file list from asset roots and include rules:

```bash
sourceweaver pack map.bsp \
  --tool /path/to/bspzip \
  --output packed.bsp \
  --asset-root /path/to/game \
  --include materials/custom/wall01.vmt \
  --include materials/custom/wall01.vtf \
  --log pack.log \
  --report pack-report.json \
  --json
```

Source Weaver resolves each `--include` as a relative Source asset path under the configured `--asset-root` directories. It writes a BSPZIP file list with path pairs:

```text
materials/custom/wall01.vmt
/path/to/game/materials/custom/wall01.vmt
materials/custom/wall01.vtf
/path/to/game/materials/custom/wall01.vtf
```

Then it runs:

```text
bspzip -addlist <input.bsp> <filelist.txt> <output.bsp>
```

The command uses the first matching asset root for each included file. Missing files are reported before the packer is launched, and the command exits non-zero after writing the JSON report.

## Tool context profiles

Some BSPZIP-compatible tools need to be launched from a game/tool `bin` directory, with local Source runtime library paths, or through a wrapper that supplies game context. Source Weaver exposes these context fields without bundling or redistributing the external packer:

```bash
sourceweaver pack map.bsp \
  --tool ./bspzip-wrapper.sh \
  --output packed.bsp \
  --asset-root /path/to/game \
  --include materials/custom/wall01.vmt \
  --context-profile explicit-game-arg-wrapper \
  --tool-cwd /path/to/game/bin \
  --library-path /path/to/game/bin \
  --game-dir /path/to/game \
  --pass-game-dir \
  --report pack-report.json \
  --json
```

`--context-profile` records the selected profile. `--tool-cwd` sets the packer working directory. Repeated `--library-path` values are prepended to `LD_LIBRARY_PATH` for the packer process and version probe. `--game-dir` records the game/content directory; `--pass-game-dir` explicitly inserts `-game <dir>` before `-addlist` for wrapper-compatible tools.

Run `sourceweaver bspzip-context-profiles --json` or see `docs/bspzip-context-profiles.md` for documented profiles and wrapper examples.

## VMF dependency discovery

Source Weaver can derive a reviewable pack list from common VMF asset references before running the packer:

```bash
sourceweaver pack map.bsp \
  --tool /path/to/bspzip \
  --output packed.bsp \
  --asset-root /path/to/game \
  --asset-root /path/to/mod \
  --discover-from-vmf merged.vmf \
  --report pack-report.json \
  --json
```

`--discover-from-vmf` can be repeated and can be combined with explicit `--include` paths. The generated BSPZIP list contains the union of explicit includes and discovered assets, de-duplicated by BSP-internal path.

The discovery pass currently recognizes these common VMF references:

- brush side material names from `material` keys, resolved as `materials/<name>.vmt`;
- material VMT texture parameters such as `$basetexture`, `$bumpmap`, `$detail`, `$envmapmask`, and related texture slots, resolved as `materials/<name>.vtf`;
- model references from `model` keys and `.mdl` values, plus existing sibling `.vvd`, `.dx80.vtx`, `.dx90.vtx`, `.sw.vtx`, and `.phy` files found under asset roots;
- explicit sound files from `message`, sound/noise keys, and `.wav`, `.mp3`, or `.ogg` values, resolved under `sound/` when needed;
- script and scene-style paths such as `scripts/...`, `.nut`, `.vcd`, `.res`, and `.cfg` values;
- explicit `particles/*.pcf` references.

Named particle systems such as `info_particle_system` `effect_name` values are reported as warnings when no explicit PCF path is present, because VMF data alone does not identify which particle manifest or PCF owns the system name.

The JSON report includes a `discovered_dependencies` object with source VMFs, asset roots, raw references, resolved assets, missing assets, ambiguous assets, and warnings. Ambiguous assets are files that exist under more than one asset root; Source Weaver uses the first configured root in the generated BSPZIP list and records a warning. Missing assets are reported before BSPZIP is launched, and the command exits non-zero after writing the report.

## Existing file lists

If another tool already generated a BSPZIP-compatible file list, pass it directly:

```bash
sourceweaver pack map.bsp \
  --tool /path/to/bspzip \
  --output packed.bsp \
  --filelist pack-list.txt \
  --json
```

`--filelist` cannot be combined with generated `--asset-root`/`--include` lists in the same command.

## Supported asset paths

Generated include paths must be relative and must not contain `..`. Backslashes are normalized to forward slashes for BSP internal paths.

Source Weaver recognizes these common Source asset roots for warnings and reporting:

- `materials`
- `models`
- `sound`
- `scripts`
- `particles`
- `resource`
- `maps`
- `cfg`
- `media`

Paths outside those roots can still be listed, but the report warns that they are outside common Source asset locations.

## Report fields

The JSON report includes:

- tool path, tool kind, and best-effort version probe
- command shape and exact command arguments
- input BSP and output BSP
- whether the output BSP exists after the run
- generated or supplied file-list path
- asset roots
- requested files with internal/external path resolution
- discovered VMF dependency details when `--discover-from-vmf` is used
- `tool_context` fields for context profile, working directory, game directory, library paths, environment keys, and context warnings
- missing files
- warnings
- exit code
- log path
- detected packed-file count when the packer prints `Adding file:` lines
- parsed warning/error summary from the packer log

## Notes and limitations

BSP packing should normally happen after a real compile and after cubemaps are built when that applies to the target game. Repacking a BSP is a distribution step, not a replacement for VMF validation or compile/runtime testing.

The current implementation covers CLI automation, machine-readable reports, and an optional desktop packing panel. Desktop packing remains an integration surface for user-provided tools and does not change VMF export or compile results retroactively.

## Sources checked

- Valve Developer Community BSPZIP search result for `-addlist <input bsp> <file list> <output bsp>` and `-addorupdatelist` command forms.
- Valve Developer Union guide: https://valvedev.info/guides/zipping-files-into-a-map-using-bspzip-or-vide/
- Hammer++ tools page for BSPZIP++ support context: https://ficool2.github.io/HammerPlusPlus-Website/tools.html

## Desktop packing panel

The desktop app exposes the same optional packing workflow in the **Optional external compile** panel under **Optional BSP packing**. The UI collects:

- user-provided BSPZIP-compatible packer tool path;
- input BSP and output BSP paths;
- asset roots and comma-separated include paths, or a filelist path;
- log path, JSON report path, and timeout seconds;
- a **Pack after compile succeeds** checkbox;
- a manual **Run BSP pack now** action.

The panel shows the exact Source Weaver CLI command, JSON report, stdout/stderr tails, missing-file count, and packed-file count when the underlying tool/report supplies one. Pack-after-compile only starts after the desktop compile worker reports success. Packing remains optional and separate from VMF export and compile success; a pack failure opens an attention dialog but does not retroactively mark VMF export as failed.

If the input BSP field is blank, the desktop action infers it from the current output VMF path by changing the extension to `.bsp`. If the output BSP or report path is blank, Source Weaver derives `*-packed.bsp` and `*-pack-report.json` paths next to the input/output BSP.

Source Weaver does not bundle BSPZIP, game content, SDKs, or custom assets. The desktop panel is an integration surface for user-configured tools only. The third-party redistribution policy keeps BSPZIP-compatible packers user-provided unless a later review approves another category.
