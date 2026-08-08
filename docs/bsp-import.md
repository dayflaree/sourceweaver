# BSP decompile import workflow

Source Weaver is VMF-first. BSP files are compiled game artifacts, while Source Weaver's parser, cleaner, previewer, and stitcher operate on editable VMF documents. BSP import therefore uses external tools to create VMFs before normal Source Weaver processing.

## Recommendation

Use BSPSource or another trusted external decompiler to generate a VMF, then import that VMF into Source Weaver.

BSPSource is the most viable current option because it is an actively maintained Source-engine BSP-to-VMF decompiler. Its project README describes it as a Java Source engine map decompiler that converts `.bsp` maps back to `.vmf` files for Hammer. BSPSource 1.4.8 was checked on 2026-08-06; its CLI accepts `-o <path>` to choose the VMF output path.

Source Weaver does not bundle BSPSource, VMEX, game BSPs, or decompiled content. The current implementation is first-class user-selected BSPSource execution plus a generic wrapper escape hatch. Managed downloads or bundled binaries require the review gate in `docs/third-party-redistribution-policy.md` before release.

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

The desktop runner delegates to the CLI `bsp-import` workflow in a background worker. Successful output is parsed, integrity-checked, imported into the selected VMF list, and tagged as BSP-derived. The UI shows the command, JSON report, stdout/stderr tails, a decompile-quality category summary, and a collapsible BSPSource warning category list. Set `SOURCEWEAVER_CLI=/path/to/sourceweaver` before launching the desktop app if the CLI executable is not next to the desktop executable.

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

Future improvements can add known BSPSource argument presets, additional fixture-backed warning categories, or legally committable tiny BSP-derived VMF fixtures. The VMF-first boundary should remain unchanged.

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

Source Weaver does not bundle BSPSource or automatically adopt latest upstream releases. See `docs/bspsource-managed-download.md` for the research result and `docs/third-party-redistribution-policy.md` for the broader managed-download review policy.


## BSPSource quality and warning categories

`sourceweaver bsp-import` parses captured BSPSource stdout/stderr into a `decompile_quality` JSON object. The parser is backed by representative legal fixture lines in `tests/fixtures/bspsource_quality.log` and stays conservative: it helps triage warnings, skipped data, unsupported data, and tool noise, while generated VMFs remain approximate and review-required.

| Category | Severity | Fatal by itself | Meaning |
| --- | --- | --- | --- |
| `tool-configuration-noise` | `info` | No | JVM, logger, Swing/AWT, or known BSPSource UI/logging noise such as `JAVA_TOOL_OPTIONS`, Log4j/SLF4J messages, or the `IsDecompileTaskFilter` console attribute line. |
| `unsupported-lump` | `warning` | No | BSPSource reported unsupported or unknown lump/game-lump data. Generated VMFs can be incomplete. |
| `skipped-data` | `warning` | No | BSPSource skipped, ignored, omitted, or discarded data while writing the VMF. |
| `quality-risk` | `warning` | No | Lines mentioning decompile protection, missing textures/models, overlays, displacements, cubemaps, pakfile/embedded file data, invalid solids, or similar review risks. |
| `decompile-warning` | `warning` | No | Generic warning lines not matched by a narrower category. |
| `tool-error` | `error` | Yes | Fatal/error/exception-like lines that may indicate a failed or incomplete decompile. |

Example JSON shape:

```json
{
  "decompile_quality": {
    "ok": true,
    "issue_count": 8,
    "errors": 0,
    "warnings": 5,
    "quality_risks": 2,
    "skipped_data": 2,
    "unsupported_lumps": 1,
    "configuration_noise": 2,
    "issues": [
      {
        "severity": "warning",
        "category": "unsupported-lump",
        "fatal": false,
        "line": 4,
        "message": "WARN Unsupported lump LUMP_OVERLAYS version 1; using fallback parser",
        "rationale": "BSPSource reported data/lump support limitations that can reduce decompile completeness"
      }
    ]
  }
}
```

The import `ok` result still depends on the external tool exit status, generated VMF existence, and Source Weaver VMF integrity checks. Non-fatal configuration noise is reported separately and is not treated as a decompile failure by itself.


## BSPSource argument presets

Raw `--tool-arg` remains available for version-specific BSPSource flags or custom wrappers. Source Weaver also provides named presets for common BSPSource CLI argument groups:

```bash
sourceweaver bsp-import-presets --json
sourceweaver bsp-import map.bsp --bspsource /path/to/bspsrc.sh --preset extract-embedded --tool-arg --custom-flag --output out.vmf --json
```

Preset arguments are applied before raw `--tool-arg` values and before the final `-o <out.vmf> <input.bsp>` BSPSource arguments. Desktop **BSP decompile import** exposes the same preset selector and keeps a **Raw tool args** field for the escape hatch.

| Preset | Arguments | Tradeoff |
| --- | --- | --- |
| `default` | none | Uses the installed BSPSource version's default behavior. |
| `extract-embedded` | `-unpack_embedded` | Extracts BSP-embedded materials/models for review; can write many files and users must still manage game/content paths manually. |
| `extract-embedded-all` | `-unpack_embedded -no_smart_unpack` | Disables smart filtering while extracting; useful for audit but can include cubemap/generated/noisy content. |
| `manual-areaportal` | `-force_manual_areaportal` | Forces manual areaportal mapping for difficult maps; generated output needs manual inspection. |
| `disable-tool-texture-fix` | `--no_ttfix` | Disables BSPSource tool texture fixups for raw-output comparison or troubleshooting. |
| `disable-cubemap-texture-fix` | `--no_cubemaptexfix` | Disables cubemap texture fixups for material-reference audit or troubleshooting. |
| `audit-raw-output` | `--no_ttfix --no_cubemaptexfix` | Disables both fixups to compare against default output; may leave more broken or noisy texture references. |

Research notes:

- The upstream README documents normal CLI use as `bspsrc -o <out.vmf> <input.bsp>`.
- BSPSource 1.4.0 release notes list `-unpack_embedded`, `-no_smart_unpack`, and `-force_manual_areaportal`.
- BSPSource 1.4.7 and 1.4.8 release notes mention CLI toggles for tool-texture and cubemap fixing, including `--no_ttfix` and `--no_cubemaptexfix` in 1.4.8 notes.
- Presets are command-construction conveniences. They are not real external-tool validation and are not guaranteed to be supported by every old BSPSource version. Use `bsp-import-presets --json` to inspect the exact arguments, and keep `--tool-arg` for local/version-specific adjustments.


## BSP-derived repository fixtures

The repository includes `tests/fixtures/bsp-derived/`, a synthetic redistributable fixture set for BSP import plumbing tests. It contains a minimal Source BSP-style header fixture, a Source Weaver-authored expected VMF, and a manifest with source, license, command, fake tool version, redaction, and checksum metadata.

The fixture set is not real BSPSource output and does not validate real decompile quality. CI uses a fake wrapper to copy the committed VMF so `sourceweaver bsp-import` can exercise command/report/integrity behavior without proprietary BSPs or external tools. See `docs/bsp-derived-fixtures.md`.


## External decompiler preset research

`sourceweaver external-decompiler-presets --json` reports the current integration status for BSPSource, VMEX, and unknown wrapper tools. VMEX is documented as legacy/documentation-only because current research found it obsolete, no longer active, not post-Orange Box compatible, and without a verified binary redistribution license. See `docs/external-decompiler-presets.md`.
