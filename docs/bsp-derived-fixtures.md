# BSP-derived fixture policy and synthetic fixture set

Source Weaver needs BSP import regression tests that can run in CI without proprietary game assets or external decompiler tools. Real game BSPs, decompiled VMFs, and embedded game content cannot be committed unless redistribution rights and provenance are verified.

## Current committed fixture set

`tests/fixtures/bsp-derived/` contains a synthetic, redistributable fixture set:

- `tiny_synthetic_header.bsp` — Source Weaver-authored binary fixture containing only a minimal Source BSP-style header: `VBSP`, version 20, zeroed lump descriptors, and map revision 0.
- `tiny_synthetic_generated.vmf` — Source Weaver-authored expected VMF used by CI fake-wrapper tests.
- `manifest.json` — source, license, checksum, command, tool, and redaction metadata.
- `README.md` — human-readable provenance and validation boundary.

The fixture is licensed as CC0-1.0 for Source Weaver-authored files and includes no Valve, game, mod, custom-map, or proprietary content.

## Why this is synthetic

During issue #103, no legally redistributable real BSP input plus real BSPSource-generated VMF pair was added to the repository. A real fixture pair needs verified redistribution terms for:

1. the BSP input;
2. any embedded or referenced assets if they are included;
3. the generated VMF;
4. the exact tool/version/command evidence;
5. any redactions.

Without that evidence, committing a real BSP-derived fixture would risk redistributing proprietary map or game data. The repository therefore uses a synthetic header fixture and fake-wrapper test to cover Source Weaver's import/report/validation plumbing while keeping real BSPSource validation outside the repo.

## CI regression coverage

The CI test `bsp_import_uses_committed_synthetic_fixture_without_external_tools` creates a fake BSPSource-compatible wrapper that:

1. accepts `--version` and reports `Source Weaver fixture wrapper 1.0`;
2. accepts the committed `tiny_synthetic_header.bsp` input;
3. copies committed `tiny_synthetic_generated.vmf` to the requested output path;
4. lets `sourceweaver bsp-import` parse, validate, and report on the generated VMF.

This verifies that the committed fixture set exercises the BSP import path without requiring proprietary BSPs, BSPSource, Java, game SDKs, game content, or network access.

## Manifest fields

`manifest.json` records:

- fixture name and source;
- license;
- BSP fixture kind;
- generated VMF kind;
- fake decompile command;
- fake tool version / no-real-tool boundary;
- redactions;
- whether real external-tool validation was performed;
- file sizes and SHA-256 checksums.

## Adding a real redistributable fixture later

A real BSP-derived fixture may be added only when all of the following are recorded:

- source URL or creation procedure for the BSP;
- license proving redistribution is allowed;
- exact BSPSource version and command line;
- generated VMF checksum;
- complete list of redactions or confirmation that none were needed;
- confirmation that no proprietary game content or custom assets are included unless redistribution is explicitly allowed;
- validation output proving the real decompiler was actually run.

Until then, real Ravenholm/HL2/BSPSource validation evidence belongs in external verification folders, not in the repository.
