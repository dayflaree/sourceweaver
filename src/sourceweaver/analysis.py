"""Initial non-mutating analysis pipeline."""

from __future__ import annotations

from pathlib import Path

from sourceweaver.model import AnalysisReport, ArtifactFingerprint, Diagnostic, Severity
from sourceweaver.vmf.document import VmfDocument

REQUIRED_TOP_LEVEL = ("versioninfo", "world")


def analyze_vmf(path: str | Path) -> AnalysisReport:
    """Perform structural analysis without modifying the VMF."""
    source = Path(path)
    document = VmfDocument.read(source)
    top_level = [block.key.value for block in document.syntax.blocks()]
    folded = {name.casefold() for name in top_level}
    diagnostics: list[Diagnostic] = []

    for required in REQUIRED_TOP_LEVEL:
        if required not in folded:
            diagnostics.append(
                Diagnostic(
                    code="VMF001",
                    severity=Severity.ERROR,
                    message=f"Required top-level block '{required}' is missing.",
                    remediation="Restore the block from the original VMF or a known-good backup.",
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

    if not any(name.casefold() == "cameras" for name in top_level):
        diagnostics.append(
            Diagnostic(
                code="VMF101",
                severity=Severity.INFO,
                message="No cameras block was found; this is legal for compiler-only VMFs.",
            )
        )

    return AnalysisReport(
        source=ArtifactFingerprint.from_path(source),
        encoding=document.encoding,
        newline="CRLF" if document.newline == "\r\n" else "LF",
        top_level_blocks=top_level,
        diagnostics=diagnostics,
        metadata={
            "token_count": len(document.syntax.tokens) - 1,
            "top_level_entry_count": len(document.syntax.entries),
            "lossless_roundtrip": document.render_bytes() == document.raw_bytes,
        },
    )
