# Rust dependency vulnerability audit

Source Weaver uses `cargo-audit` for Rust dependency vulnerability checks.

RustSec documents `cargo-audit` as a tool that audits `Cargo.lock` files for crates with security vulnerabilities in the RustSec Advisory Database. The CI workflow installs `cargo-audit` and runs `cargo audit` on every push and pull request.

## Local command

```bash
cargo install cargo-audit --locked
cargo audit
```

A passing audit must exit with status 0 before a public release is tagged.

## Current audit result

Last local run: 2026-08-07 on OldBeast.

```text
cargo-audit-audit 0.22.2
cargo audit
Loaded 1190 security advisories
Scanning Cargo.lock for vulnerabilities (445 crate dependencies)
warning: 2 allowed warnings found
```

The audit found no vulnerability errors after updating the desktop UI dependency set from `eframe 0.31` / `rfd 0.15` to `eframe 0.32` / `rfd 0.16`, which removed vulnerable `quick-xml 0.30.0` from the lockfile.

## Documented warnings

The current audit reports two unmaintained transitive dependency warnings:

| Crate | Advisory | Current path | Decision |
| --- | --- | --- | --- |
| `paste 1.0.15` | `RUSTSEC-2024-0436` | Present in `Cargo.lock` through `metal 0.31.0` / `wgpu-hal 25.0.2` from the desktop `eframe` lock graph. `cargo tree --workspace --target all` does not show an active workspace path, but `cargo audit` audits the whole lockfile and reports it. | Accepted as a lockfile warning for now. Keep auditing every CI run and remove when upstream `eframe`/`wgpu` no longer leaves it in the lockfile. |
| `ttf-parser 0.25.1` | `RUSTSEC-2026-0192` | `owned_ttf_parser` / `ab_glyph` through `egui` and `winit` desktop font/rendering dependencies | Accepted as a transitive warning for now. Keep auditing every CI run and remove when upstream `egui`/`winit` no longer depends on it. |

These warnings do not currently make `cargo audit` fail. New vulnerability errors must be fixed or documented with a release-blocking decision before release.

## Release evidence to record

For every public release, record:

- `cargo audit --version`;
- `cargo audit` exit status and output summary;
- any vulnerability errors and the fix or release-blocking decision;
- any unmaintained/yanked warnings that remain accepted;
- the commit hash and CI run URL for the audit gate.

## Redistribution boundary

`cargo-audit` and the RustSec advisory database are build/release validation tools. Source Weaver release artifacts do not redistribute `cargo-audit`, advisory database contents, or third-party source packages.
