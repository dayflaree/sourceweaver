# BSP content packing workflow

Source Weaver can run an optional BSP packing step with a user-provided `bspzip`, BSPZIP++, or compatible external tool. Packing is separate from VMF editing, VMF merging, BSP decompiling, and VMF-to-BSP compiling.

Source Weaver does not bundle Valve tools, third-party packers, compiled BSPs, or custom assets. Users are responsible for owning or having permission to distribute every asset they pack.

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

- tool path and tool kind
- command shape and exact command arguments
- input BSP and output BSP
- whether the output BSP exists after the run
- generated or supplied file-list path
- asset roots
- requested files with internal/external path resolution
- missing files
- warnings
- exit code
- log path
- detected packed-file count when the packer prints `Adding file:` lines
- parsed warning/error summary from the packer log

## Notes and limitations

BSP packing should normally happen after a real compile and after cubemaps are built when that applies to the target game. Repacking a BSP is a distribution step, not a replacement for VMF validation or compile/runtime testing.

The current implementation covers CLI automation and machine-readable reports. Desktop post-compile packing UI remains future work once the compile workflow is stable.

## Sources checked

- Valve Developer Community BSPZIP search result for `-addlist <input bsp> <file list> <output bsp>` and `-addorupdatelist` command forms.
- Valve Developer Union guide: https://valvedev.info/guides/zipping-files-into-a-map-using-bspzip-or-vide/
