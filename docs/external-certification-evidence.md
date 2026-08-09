# External certification evidence bundles

Use `scripts/validate-external-certification-evidence.sh` before closing any ticket that depends on manual external evidence from an editor, compiler, model viewer, game runtime, or signing system.

The validator checks evidence completeness and redaction structure. It does not prove that an external Source tool succeeded. The issue comment still needs the exact commands, tool versions, artifact hashes, CI status, validated claims, unvalidated claims, and redistribution/legal boundary.

## Command

```bash
scripts/validate-external-certification-evidence.sh /tmp/sourceweaver-completion-evidence/issue145-runtime-map-load
```

Run the built-in fixture checks with:

```bash
scripts/validate-external-certification-evidence.sh --self-test
```

## Required bundle layout

```text
evidence-manifest.json
SHA256SUMS
tool-versions.txt
commands.sh
validation-summary.md
legal-boundary.md
```

`SHA256SUMS` must list every required file except `SHA256SUMS` itself. It must also list each artifact named by the manifest. All listed paths must be relative to the bundle directory.

## Manifest fields

```json
{
  "issue": 145,
  "sourceweaver_commit": "0123456789abcdef0123456789abcdef01234567",
  "tool_kind": "runtime",
  "host_os": "Windows 11 Pro 24H2",
  "external_tool_versions": ["example tool version output"],
  "commands": ["example command line"],
  "artifacts": ["runtime-log.txt"],
  "validated_claims": ["what this evidence supports"],
  "unvalidated_claims": ["what this evidence leaves unresolved"],
  "redistribution_boundary": "Synthetic or legally owned artifacts only; no proprietary content, private assets, secrets, certificates, or signing keys."
}
```

Accepted `tool_kind` values are `runtime`, `hammer`, `compiler`, `hlmv`, and `signing`.

## Redaction checks

The validator scans required files and small hashed artifacts for obvious private host path patterns, including Linux home directories, macOS home directories, Windows user profiles, `/root`, `~/...`, and `$HOME/...`.

Sanitize evidence before storing it. Use bundle-relative artifact names and summaries instead of raw local paths. Keep proprietary game files, Steam files, Source SDK binaries, Hammer or HLMV binaries, BSPs, maps without redistribution rights, private models/materials/assets, screenshots with private content, private logs, signing keys, certificates, passphrases, GitHub secret values, and production signing credentials outside the repository.

## Claim guard

After bundle checks finish, the script runs:

```bash
python3 scripts/check-validation-claims.py
```

This keeps docs and release notes aligned with the current evidence boundary after manual rows are added.

## Fixture coverage

Synthetic fixtures live under `tests/fixtures/external-certification-evidence/`:

- `valid-runtime` exercises the accepted bundle shape.
- `missing-hash` omits a required file from `SHA256SUMS` and must fail.
- `private-path` contains a raw home-directory path and must fail.
