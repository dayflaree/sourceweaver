# Repository audit — August 5, 2026

## Scope

This audit covers every feature currently implemented in SourceWeaver 0.1.0:

- lossless VMF lexing and concrete-syntax parsing;
- bounded file, character, token, and nesting-depth handling;
- encoding and newline preservation;
- source-span patch application;
- structural VMF analysis and JSON report validation;
- non-destructive round-trip output;
- GMod installation and compiler discovery;
- compiler fingerprinting and executable-format classification;
- read-only CST-backed semantic entity and targetname graph extraction;
- read-only convex brush geometry reconstruction, relation classification, and blocker diagnostics;
- project-skill installation;
- CLI entry points;
- packaging, dependency metadata, and CI.

Map stitching, FGD-backed semantic mutation, geometry transformations, compiler execution, runtime validation, and visibility optimization remain roadmap features. They are excluded from the functionality claim because no qualified implementation exists yet.

## Defects found and corrected

1. `roundtrip` could overwrite its input or an existing output. Input identity is now checked across literal paths, resolved paths, symlinks, and hard links. Existing output requires `--force`; the source VMF remains immutable.
2. VMF parsing had no implemented resource limits. File bytes, decoded characters, tokens, and nesting depth are now bounded with clean user-facing failures.
3. VMF reads and executable fingerprints did not detect every mid-read path replacement. Descriptor and current-path identities are now compared before a result is accepted. Native Windows CI then showed that device, inode, and creation/change-time fields are not consistently represented across `stat()` and `fstat()` on every Windows filesystem; Windows now compares the cross-API-stable size and last-write time fields, while POSIX retains the stronger device/inode/size/mtime/ctime identity.
4. GMod discovery accepted arbitrary existing directories. A valid install root now requires both `bin` and `garrysmod`, or a valid inner `garrysmod/gameinfo.txt` path with a parent `bin` directory.
5. Steam discovery missed legacy `libraryfolders.vdf` syntax, secondary libraries, Debian-native, Flatpak, and Snap Steam roots. These cases now have platform-neutral regression tests.
6. Compiler discovery could silently fall back to an unrelated executable on `PATH`. PATH fallback is now opt-in.
7. `doctor` repeatedly fingerprinted the full toolchain and did not report executable/host compatibility. It now fingerprints once and reports PE/ELF/Mach-O compatibility explicitly.
8. Generated output writes and skill installation had incomplete failure recovery. Output writes are non-destructive and atomic where the filesystem permits; failed staged skill replacement restores the previous installation.
9. The CLI lacked `python -m sourceweaver` support and clean error handling for malformed files. Both are implemented and tested.
10. Published JSON schema newline values did not cover CR-only, mixed, or no-newline files. The schema and validation tests now match runtime reports.
11. The source distribution included generated Hypothesis cache data. Generated state is excluded and clean source/wheel archives are verified.
12. The installed-wheel smoke test depended on stdlib `venv` and failed on Debian Python builds without `ensurepip`. It now uses the cross-platform `virtualenv` package on both Windows and Linux.
13. Documentation overstated completed `srctools` integration. It now identifies the semantic adapter as roadmap work.

## Validation performed

### Static and structural gates

- Ruff formatting: pass.
- Ruff lint: pass.
- strict MyPy: pass.
- Bandit source/security scan: pass.
- Vulture dead-code scan at 80% confidence: pass.
- Python `compileall`: pass.
- `uv lock --check`: pass.
- installed dependency consistency: pass.
- JSON Schema Draft 2020-12 validation: pass.

### Tests

- 116 tests pass locally on Linux.
- Branch-aware coverage is 90.13%, above the enforced 80% floor.
- Property-based VMF round-trip tests use Hypothesis.
- The prior merged cross-platform baseline passed locally on Python 3.11, 3.12, 3.13, and 3.14.
- GitHub Actions runs the test suite on `ubuntu-latest` and `windows-latest` for Python 3.11–3.14; all thirteen quality, test, package, dependency, and documentation jobs passed for the prior cross-platform baseline in [run 30241981184](https://github.com/dayflaree/sourceweaver/actions/runs/30241981184). Current PRs must pass that matrix before merge.

### Packaging and dependencies

- Wheel build: pass.
- Source distribution build: pass.
- Twine metadata validation: pass.
- Clean installed-wheel smoke test: pass.
- Console entry point and `python -m sourceweaver`: pass.
- Runtime dependency vulnerability scan with `pip-audit`: no known vulnerabilities found.
- Generated cache files and proprietary game/compiler artifacts are absent from distributions.

## Supported platform statement

The implemented Python foundation is supported on Windows and Linux with Python 3.11–3.14. Platform-specific compiler execution remains unimplemented. On Linux, discovery can identify Windows PE Tools++ executables and reports that a compatibility layer is required; it does not claim that Wine or Proton invocation has been qualified.

## Confidence boundary

The current implemented surface has automated coverage and clean validation results. Absolute correctness for every possible VMF byte sequence, filesystem, Steam installation, future Python release, or future GMod/Tools++ build cannot be guaranteed. Unsupported or ambiguous conditions fail closed. Roadmap features must pass their own compiler and runtime qualification gates before they become part of the supported functionality claim.
