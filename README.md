# SourceWeaver

[![CI](https://github.com/dayflaree/sourceweaver/actions/workflows/ci.yml/badge.svg)](https://github.com/dayflaree/sourceweaver/actions/workflows/ci.yml)

**Automation-first VMF analysis, map stitching, and compiler-verified optimization for Source 1 and Garry's Mod.**

SourceWeaver is a pre-alpha engineering project for automating repetitive Hammer++ map work while keeping geometry changes deterministic, reversible, and proven by the exact game compiler/runtime being targeted.

The end-state workflow is:

```text
VMF inputs
  -> lossless parse and immutable fingerprints
  -> semantic + geometric analysis
  -> candidate transformations with evidence
  -> generated worktree and patch manifest
  -> VBSP++ / VVIS++ / VRAD++ validation
  -> automated GMod runtime scenarios
  -> measured comparison and review report
  -> optional approval and export
```

## Project promise

SourceWeaver aims for **little or no manual construction work**. Human work is limited to reviewing evidence and approving changes. The tool will refuse to auto-apply a transformation when it cannot prove its safety inside the supported envelope.

No software can guarantee that every arbitrary VMF, decompiled BSP, custom entity, Lua addon, VScript, or undocumented engine branch will behave perfectly. SourceWeaver therefore turns uncertainty into explicit gates:

- byte-identical source preservation before any mutation;
- typed entity-reference rewriting;
- exact geometric predicates for committed edits;
- compiler acceptance using fingerprinted executables;
- metric comparison against an untouched baseline;
- runtime scenario verification;
- automatic rollback on any regression or unknown condition.

See [Automation Contract](docs/AUTOMATION_CONTRACT.md).

## Current status

The repository currently provides the verified foundation:

- a lossless VMF/KeyValues lexer and concrete syntax tree;
- exact preservation of comments, duplicate keys, whitespace, line endings, encodings, and unknown blocks;
- conflict-checked source edits;
- VMF structural inspection;
- Steam-library-aware GMod compiler discovery and stable SHA-256 fingerprinting;
- byte-identical round-trip fixtures with bounded parser depth, size, and token counts;
- Windows and Linux CI across Python 3.11–3.14;
- clean wheel/source-distribution builds and installed-wheel smoke tests;
- complete research, architecture, validation, and implementation plans;
- reusable project skills under [`skills/`](skills/README.md).

Map stitching and visibility transformations remain roadmap work. They are deliberately gated behind the source-integrity and compiler-validation layers.

## Quick start

```bash
python -m venv .venv
# Windows: .venv\Scripts\activate
# Linux/macOS: source .venv/bin/activate
python -m pip install -e ".[dev]"

sourceweaver inspect path/to/map.vmf
sourceweaver roundtrip path/to/map.vmf --output .work/map.roundtrip.vmf
sourceweaver doctor --gmod-root "C:/Program Files (x86)/Steam/steamapps/common/GarrysMod"
pytest
```

## Why a lossless parser is required

VMF is text, but a conventional semantic parser is insufficient for editor-safe automation. Existing parsers generally reconstruct known structures when exporting. That can change formatting and may discard unknown Hammer++ or future editor data. SourceWeaver keeps a concrete syntax tree and applies span-based patches so untouched text remains exactly untouched.

`srctools` remains the planned semantic Source-format library for FGD, VMF, BSP, VPK, material, instance, and entity-I/O support. The semantic adapter is roadmap work and is available to developers through the optional `semantics` dependency group.

## Major systems

| System | Responsibility |
|---|---|
| Lossless VMF layer | Preserve source bytes, unknown blocks, comments, duplicate outputs, and formatting |
| Semantic model | FGD-typed entities, references, I/O, assets, instances, and game-branch rules |
| Geometry kernel | Convex brush reconstruction, robust predicates, adjacency, intersection, and fitting |
| Spatial model | Occupied/empty-space graphs, doors, choke points, rooms, exterior, and reachability |
| Stitch planner | Landmark alignment, overlap classification, namespacing, controller reconciliation, lifecycle synthesis |
| Visibility optimizer | Nodraw/detail candidates, areaportal proofs, hint search, PVS/runtime metrics |
| Compiler harness | Fingerprinted VBSP/VVIS/VRAD/BSPZIP execution, logs, BSP/PRT inspection, reproducibility |
| Runtime harness | GMod scenario runs, console assertions, trigger checks, PVS samples, screenshots, regression verdicts |
| AI planner | Natural-language intent, candidate ranking, explanations, and ambiguity handling |

## Delivery backlog

Implementation work is organized under the [verified two-map stitcher milestone](https://github.com/dayflaree/sourceweaver/milestone/1) and the [issue tracker](https://github.com/dayflaree/sourceweaver/issues). Every issue defines deliverables and a measurable exit gate.

## Documentation

Start at [`docs/INDEX.md`](docs/INDEX.md). Important documents include:

- [Research report](docs/RESEARCH.md)
- [Claim and confidence ledger](docs/RESEARCH_LEDGER.md)
- [Architecture](docs/ARCHITECTURE.md)
- [VMF integrity](docs/VMF_INTEGRITY.md)
- [Map stitching](docs/MAP_STITCHING.md)
- [Campaign lifecycle](docs/CAMPAIGN_LIFECYCLE.md)
- [Visibility optimization](docs/VISIBILITY_OPTIMIZATION.md)
- [Compiler validation](docs/COMPILER_VALIDATION.md)
- [Runtime validation](docs/RUNTIME_VALIDATION.md)
- [Failure modes](docs/FAILURE_MODES.md)
- [Roadmap](docs/ROADMAP.md)

## Research date and branch awareness

The research baseline was refreshed on **July 26, 2026**. Source and tool behavior changes over time. Every accepted build report records exact executable hashes, configuration, command lines, source VMF hashes, and relevant tool versions.

## Content and compiler policy

- Do not commit or redistribute Valve maps, decompiled campaign VMFs, game assets, or proprietary compiler binaries.
- Use synthetic fixtures and locally supplied legal inputs.
- Hammer++ Tools++ explicitly requests permission before redistribution of its map compilers. SourceWeaver discovers locally installed tools and never vendors them.
- Patch recipes should contain transformations and hashes, not copyrighted map content.

## License

SourceWeaver code and original documentation are MIT licensed. Third-party projects retain their own licenses.
