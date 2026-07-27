"""Initial non-mutating analysis pipeline."""

from __future__ import annotations

from collections import Counter
from pathlib import Path

from sourceweaver.model import AnalysisReport, ArtifactFingerprint, Diagnostic, Severity
from sourceweaver.vmf.document import VmfDocument
from sourceweaver.vmf.parser import PairNode

REQUIRED_TOP_LEVEL = ("versioninfo", "world")


def _pair_value(block_entries: tuple[object, ...], key: str) -> str | None:
    folded = key.casefold()
    for entry in block_entries:
        if isinstance(entry, PairNode) and entry.key.value.casefold() == folded:
            return entry.value.value
    return None


def analyze_vmf(path: str | Path) -> AnalysisReport:
    """Perform structural analysis without modifying the VMF."""
    source = Path(path)
    document = VmfDocument.read(source)
    top_level_blocks = document.syntax.blocks()
    top_level = [block.key.value for block in top_level_blocks]
    counts = Counter(name.casefold() for name in top_level)
    diagnostics: list[Diagnostic] = []

    for required in REQUIRED_TOP_LEVEL:
        count = counts[required]
        if count == 0:
            diagnostics.append(
                Diagnostic(
                    code="VMF001",
                    severity=Severity.ERROR,
                    message=f"Required top-level block '{required}' is missing.",
                    remediation="Restore the block from the original VMF or a known-good backup.",
                )
            )
        elif count > 1:
            diagnostics.append(
                Diagnostic(
                    code="VMF003",
                    severity=Severity.ERROR,
                    message=f"Top-level block '{required}' appears {count} times.",
                    remediation="Keep exactly one authoritative block before compiling.",
                )
            )

    if document.render_bytes() != document.raw_bytes:
        diagnostics.append(
            Diagnostic(
                code="VMF002",
                severity=Severity.BLOCKER,
                message="Lossless round-trip invariant failed.",
                remediation="Do not transform this file; file an issue with a redacted fixture.",
            )
        )

    world_blocks = document.syntax.blocks("world")
    if len(world_blocks) == 1:
        classname = _pair_value(world_blocks[0].entries, "classname")
        if classname is None or classname.casefold() != "worldspawn":
            diagnostics.append(
                Diagnostic(
                    code="VMF004",
                    severity=Severity.ERROR,
                    message="The world block does not declare classname 'worldspawn'.",
                    remediation="Restore the worldspawn classname before compiling.",
                )
            )

    if not any(name.casefold() == "cameras" for name in top_level):
        diagnostics.append(
            Diagnostic(
                code="VMF101",
                severity=Severity.INFO,
                message="No cameras block was found; this is legal for compiler-only VMFs.",
            )
        )

    root_pairs = [entry for entry in document.syntax.entries if isinstance(entry, PairNode)]
    if root_pairs:
        diagnostics.append(
            Diagnostic(
                code="VMF102",
                severity=Severity.WARNING,
                message=f"Found {len(root_pairs)} key/value entries outside top-level blocks.",
                evidence=[entry.key.value for entry in root_pairs[:10]],
                remediation="Verify that the entries are intentional editor extensions.",
            )
        )

    return AnalysisReport(
        source=ArtifactFingerprint.from_bytes(document.raw_bytes, path=source),
        encoding=document.encoding,
        newline=document.newline_style,
        top_level_blocks=top_level,
        diagnostics=diagnostics,
        metadata={
            "token_count": len(document.syntax.tokens) - 1,
            "top_level_entry_count": len(document.syntax.entries),
            "lossless_roundtrip": document.render_bytes() == document.raw_bytes,
        },
    )
