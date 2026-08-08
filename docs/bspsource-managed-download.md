# Managed BSPSource download and checksum policy

Source Weaver supports user-selected BSPSource launcher, jar, and wrapper paths. Those local paths remain the primary integration path. The managed BSPSource helper adds a conservative optional workflow for users who want Source Weaver to report a pinned upstream manifest, verify a local ZIP, or perform an explicit checksum-verified cache download.

Source Weaver does not bundle BSPSource, Java runtimes, game SDKs, game content, BSP files, or decompiled outputs. `docs/third-party-redistribution-policy.md` defines the wider review gate for managed downloads and redistributable third-party assets.

## Research summary

Research performed for issue #100 checked upstream BSPSource metadata from `ata4/bspsrc`:

- Upstream `LICENSE.md` lists BSPSource itself under the Unlicense.
- The same license file lists Apache Log4j 2, Apache Commons Compress, picocli, FlatLaf, and jSystemThemeDetector under Apache License 2.0.
- The same license file lists MigLayout under the 3-Clause BSD license.
- The GitHub release API for BSPSource `v1.4.8` publishes SHA-256 digests for the release ZIP assets.
- Upstream README says `bspsrc-linux.zip` and `bspsrc-windows.zip` are selected by operating system, while `bspsrc-jar-only.zip` works on systems with Java 24+.

This review supports user-initiated download/cache of upstream assets with checksum verification. It does not justify bundling BSPSource inside Source Weaver release artifacts, and it does not replace user-selected local tool paths.

## Pinned manifest

Source Weaver pins BSPSource `v1.4.8`.

| Asset | Platform | Size | SHA-256 |
| --- | --- | ---: | --- |
| `bspsrc-jar-only.zip` | portable Java | 7,414,395 | `d5effc38b78c4f60f8eb4f9be1db717bb808227a9013f82d20f34860a128b0e7` |
| `bspsrc-linux.zip` | Linux | 49,422,392 | `646c3dcc7cdc58650a96ad985a0e093bf3ef1e53b43e01aae01168910d14a32d` |
| `bspsrc-windows.zip` | Windows | 45,781,672 | `6297f7fa567adbaf72738b0a707ff45916edc920c66543098408e4b9d41ec4a9` |

Manifest command:

```bash
sourceweaver bspsource manifest --json
```

Policy command:

```bash
sourceweaver bspsource policy --json
```

## Cache policy

Cache lookup uses:

1. `SOURCEWEAVER_TOOL_CACHE` when set;
2. on Linux/macOS-like systems, `$HOME/.cache/sourceweaver/tools`;
3. on Windows, `%LOCALAPPDATA%/SourceWeaver/tools`;
4. `.sourceweaver-tools` when no per-user directory is discoverable.

Assets are stored under:

```text
<cache>/bspsource/v1.4.8/<asset-name>.zip
```

Report a cache path without downloading:

```bash
sourceweaver bspsource cache-path --asset linux --cache-dir /tmp/sourceweaver-tools --json
```

## Checksum verification

Verify a local ZIP against a pinned asset:

```bash
sourceweaver bspsource verify --asset linux --file /path/to/bspsrc-linux.zip --json
```

Verify any local file against an explicit SHA-256 digest:

```bash
sourceweaver bspsource verify --file fixture.bin --sha256 <64-hex-digest> --json
```

The asset verifier checks both SHA-256 and expected size. Explicit digest verification checks SHA-256 only.

## Download policy

Managed download is intentionally explicit:

```bash
sourceweaver bspsource download \
  --asset linux \
  --cache-dir /tmp/sourceweaver-tools \
  --accept-download-policy \
  --json
```

The command downloads from the pinned GitHub release URL, writes a `.partial` file, verifies size and SHA-256, then renames it into the versioned cache. Without `--accept-download-policy`, the command refuses to download and reminds the user that local BSPSource tool paths are supported.

Source Weaver does not automatically track the latest BSPSource release. Updating the pinned version requires a Source Weaver code/docs change with the new version, release URL, asset URLs, sizes, SHA-256 digests, and renewed license/provenance review under `docs/third-party-redistribution-policy.md`.

## Execution boundary

Managed download only obtains and verifies the upstream ZIP. It does not extract the ZIP, run BSPSource, decompile BSPs, validate decompile quality, launch Hammer, or load game content.

Real BSPSource validation is only claimed when the real tool is actually run and evidence is recorded. Existing `bsp-import` options for user-provided launchers, jars, and wrappers remain supported.
