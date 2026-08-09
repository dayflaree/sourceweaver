# Rust dependency vulnerability audit

Source Weaver uses `cargo-audit` for Rust dependency vulnerability checks.

RustSec documents `cargo-audit` as a tool that audits `Cargo.lock` files for crates with security vulnerabilities in the RustSec Advisory Database. The CI workflow installs `cargo-audit` and runs `scripts/cargo-audit-final-release.sh` on every push and pull request.

## Local command

```bash
cargo install cargo-audit --locked
cargo audit
scripts/cargo-audit-final-release.sh
```

`cargo audit` shows the full advisory report. `scripts/cargo-audit-final-release.sh` runs `cargo audit --deny warnings` with only the documented advisory IDs ignored, so a new vulnerability, yanked-crate warning, or unapproved unmaintained advisory fails the release gate.

## Current audit result

Last local run: 2026-08-08 on OldBeast.

```text
cargo-audit-audit 0.22.2
cargo audit
Loaded 1190 security advisories
Scanning Cargo.lock for vulnerabilities (457 crate dependencies)
warning: 2 allowed warnings found
```

```text
scripts/cargo-audit-final-release.sh
cargo audit --deny warnings --ignore RUSTSEC-2024-0436 --ignore RUSTSEC-2026-0192
exit status: 0
```

The audit found no vulnerability errors after updating the desktop UI dependency set from `eframe 0.31` / `rfd 0.15` to `eframe 0.32` / `rfd 0.16`, which removed vulnerable `quick-xml 0.30.0` from the lockfile.

## Documented warnings

The current audit reports two unmaintained transitive dependency warnings:

| Crate | Advisory | Affected version | Dependency path | Classification | Decision and revisit trigger |
| --- | --- | --- | --- | --- | --- |
| `paste` | `RUSTSEC-2024-0436` | `1.0.15` | Present in `Cargo.lock` through `metal 0.31.0` / `wgpu-hal 25.0.2` from the desktop `eframe` lock graph. `cargo tree --workspace --target all -i paste` prints no active workspace path, so this is currently a lockfile-only/macOS graphics-stack residue rather than a Linux or Windows runtime path. | Transitive desktop GUI lockfile warning; no direct Source Weaver runtime callsite. | Accepted only through the narrow `scripts/cargo-audit-final-release.sh` ignore for this advisory ID. Remove the ignore when upstream `eframe`/`wgpu` drops `paste`, when `cargo tree --target all -i paste` shows an active path that can be removed, or when RustSec changes the advisory from unmaintained to vulnerability/unsoundness. |
| `ttf-parser` | `RUSTSEC-2026-0192` | `0.25.1` | `owned_ttf_parser` / `ab_glyph` through `egui` and `winit` desktop font/rendering dependencies. `cargo tree --workspace --target all -i ttf-parser` confirms the path through `sourceweaver-desktop`. | Transitive desktop GUI font/rendering dependency; not used by CLI merge/validation paths. | Accepted only through the narrow `scripts/cargo-audit-final-release.sh` ignore for this advisory ID. Remove the ignore when upstream `egui`/`winit` moves to a maintained parser, when Source Weaver can feature-gate the path out of release builds, or when RustSec changes the advisory from unmaintained to vulnerability/unsoundness. |

Plain `cargo audit` reports these as allowed warnings. The final-release wrapper fails on every warning except the two advisory IDs documented above, which keeps the allowlist narrow and auditable. On 2026-08-08, `cargo update -p paste` and `cargo update -p ttf-parser` both reported `Locking 0 packages`; no safe lockfile-only upgrade removed the advisories. New vulnerability errors, yanked crates, unsoundness warnings, or additional unmaintained advisories must be fixed or handled in a separate release-blocking decision before release.

## Release evidence to record

For every public release, record:

- `cargo audit --version`;
- `cargo audit` exit status and output summary;
- `scripts/cargo-audit-final-release.sh` exit status and output summary;
- any vulnerability errors and the fix or release-blocking decision;
- any unmaintained/yanked warnings that remain accepted;
- the commit hash and CI run URL for the audit gate.

## Redistribution boundary

`cargo-audit` and the RustSec advisory database are build/release validation tools. Source Weaver release artifacts do not redistribute `cargo-audit`, advisory database contents, or third-party source packages.
