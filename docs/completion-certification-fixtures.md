# Completion certification fixtures and evidence workspace

Source Weaver completion certification uses repository-owned synthetic fixture sources plus a non-repository evidence workspace under `/tmp`. The workspace is for manual external evidence gathered for #141, #142, #145, and #146.

Generate the workspace with:

```bash
SOURCEWEAVER_COMPLETION_EVIDENCE_OVERWRITE=1 \
  scripts/prepare-completion-evidence-workspace.sh /tmp/sourceweaver-completion-evidence
```

The script creates:

```text
/tmp/sourceweaver-completion-evidence/
  issue141-hammer-open-save/
  issue142-windows-native-compile/
  issue145-runtime-map-load/
  issue146-hlmv-render/
  manifests/
  redacted-summaries/
  README.md
  SHA256SUMS
  sourceweaver-cert-room-validation.json
```

Each issue directory is a seed evidence bundle with the required #160 bundle files:

```text
evidence-manifest.json
SHA256SUMS
tool-versions.txt
commands.sh
validation-summary.md
legal-boundary.md
input/...
```

The generator validates each issue bundle with `scripts/validate-external-certification-evidence.sh` and validates the synthetic VMF through Source Weaver with the HL2 rule-set. The seed bundles document what is prepared and what is still unresolved. Replace placeholder external tool-version and command notes with exact sanitized evidence before closing the dependent issues.

## Repository fixture sources

```text
tests/fixtures/completion-certification/PROVENANCE.md
tests/fixtures/completion-certification/vmf/sourceweaver-cert-room.vmf
tests/fixtures/completion-certification/model-source/models/sourceweaver/sw_cert_cube.qc
tests/fixtures/completion-certification/model-source/models/sourceweaver/sw_cert_cube_ref.smd
tests/fixtures/completion-certification/model-source/models/sourceweaver/sw_cert_cube_idle.smd
tests/fixtures/completion-certification/model-source/materials/models/sourceweaver/synthetic_checker.vmt
```

`sourceweaver-cert-room.vmf` is a small synthetic room with six world brushes, one player start, one light, and one `info_landmark`. The model source package is original QC/SMD/VMT text for a simple cube. No compiled model outputs are committed.

## Legal boundary

The committed fixtures are Source Weaver-authored text files under the repository license. They contain no proprietary map data, Steam files, Source SDK binaries, Hammer/Hammer++ binaries, HLMV/HLMV++ binaries, VBSP/VVIS/VRAD binaries, BSPs, MDLs, VTFs, private assets, screenshots, private logs, signing keys, certificates, passphrases, GitHub secret values, or production signing credentials.

Generated BSPs, compiled model outputs, external tool binaries, screenshots, and raw logs belong in the `/tmp` workspace or another external evidence location. Commit only sanitized scripts, docs, tests, and redacted summaries.

## Validation boundary

This workflow prepares legal fixture sources and evidence directories. It does not record real external editor, compiler, model viewer, game runtime, or signing evidence. The dependent issues remain responsible for the actual manual evidence rows:

- #141 records real editor open-save evidence.
- #142 records native Windows compiler evidence.
- #145 records game-runtime map-load evidence.
- #146 records model viewer render evidence.
