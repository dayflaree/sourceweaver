# Compatibility and validation matrix

This matrix records what Source Weaver can honestly claim for preview releases. A row marked `not validated` is an explicit no-claim boundary: users must not treat that compatibility area as certified until a future issue records real evidence.

| Area | Current status | Evidence | Release wording |
| --- | --- | --- | --- |
| VMF parser, merge, transform, and integrity checks | validated with fixtures, golden snapshots, public VMFs, and CI | `cargo test --workspace`; `scripts/validate-public-vmfs.sh`; `docs/real-vmf-validation.md` | Source Weaver validates VMF text structure, preservation rules, and its own merge/cleanup behavior. |
| Hammer/Hammer++ open/save | not validated | #131 blocker evidence found no runnable Hammer/Hammer++ executable in the current environment; `docs/hammer-validation-workflow.md`; future certification issue #141 | `Hammer/Hammer++ open/save: not certified.` Do not call generated VMFs Hammer-certified, Hammer-compatible, or editor-open validated until #141 or a successor issue records a completed real-editor row. |
| Real VBSP/VVIS/VRAD compiler rows | partially validated only for completed matrix rows | `docs/source-compiler-smoke-test-matrix.md` | Name the exact completed row/toolchain. Do not generalize wrapper rows into native Windows or broad Source compiler certification. |
| Native Windows Source compiler execution | not validated | #133 blocker evidence; `docs/source-compiler-smoke-test-matrix.md` Row B; future issue #142 | `Native Windows Source compiler execution: not certified.` CI Windows Rust build/test coverage, Windows release packaging, and Proton/Wine wrapper rows are separate and must not be described as native Windows compiler validation. |
| Game-runtime map load | failure evidence only | `docs/runtime-map-load-validation.md`; future issue #145 | Existing evidence records a real runtime launch failure before successful map load. Do not claim playable map-load success. |
| Rendered HLMV/HLMV++ model preview | failure evidence only | `docs/model-tooling.md`; future issue #146 | Existing evidence records real viewer launch plumbing and failure before a rendered window opened. Do not claim rendered preview success. |
| Production release signing | not configured for a public release | `docs/code-signing.md`; future issue #143 | Release artifacts are unsigned unless that release records real signing credentials and verification output. |
| Automatic update install/rollback | not implemented | `docs/update-strategy.md`; future issue #154 | Signed update checks, verified downloads, and manual install handoff exist. Automatic installer execution, executable replacement, silent install, and rollback are not enabled. |

## Hammer/Hammer++ no-claim boundary

Source Weaver exports VMF text intended for downstream Source editor/compiler workflows, but current validation is portable Source Weaver validation only. It checks VMF structure, preservation rules, ID handling, merge behavior, preview extraction, rule-set warnings, and captured compiler-log parsing. It does not prove that Hammer or Hammer++ can open a generated VMF, preserve it on save, avoid editor warnings, or retain all editor metadata exactly.

A Hammer/Hammer++ compatibility row becomes claimable only after a real editor executable opens a generated VMF, saves a separate copy, and the evidence includes tool path/version, game configuration, warnings/dialogs/logs, input/output hashes, original-vs-saved diff summary, Source Weaver validation of the saved VMF, and redistribution boundaries. #141 tracks that future work and must stay open until those criteria are met.

## Release-note guard

Run the automated guard before publishing preview release notes:

```bash
python3 scripts/check-validation-claims.py --self-test
python3 scripts/check-validation-claims.py
```

The guard rejects unsupported phrases such as `Hammer-compatible` or `Hammer/Hammer++ open/save passed` unless a future allowlist entry points to a completed real-editor evidence row. Until then, preview release notes must include the exact limitation line:

```text
Hammer/Hammer++ open/save: not certified.
```
