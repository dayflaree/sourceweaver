"""Deterministic, conflict-checked source patching."""

from __future__ import annotations

from dataclasses import dataclass

from sourceweaver.errors import PatchConflictError


@dataclass(frozen=True, order=True, slots=True)
class TextEdit:
    """Replace the half-open source span ``[start, end)`` with *replacement*."""

    start: int
    end: int
    replacement: str
    reason: str = ""

    def __post_init__(self) -> None:
        if self.start < 0 or self.end < self.start:
            raise ValueError(f"Invalid edit span [{self.start}, {self.end})")


def apply_edits(text: str, edits: tuple[TextEdit, ...] | list[TextEdit]) -> str:
    """Apply non-overlapping edits and preserve all untouched text exactly."""
    ordered = sorted(edits, key=lambda edit: (edit.start, edit.end))
    previous_end = 0
    for edit in ordered:
        if edit.end > len(text):
            raise ValueError(f"Edit ends outside source text: {edit.end} > {len(text)}")
        if edit.start < previous_end:
            raise PatchConflictError(
                f"Overlapping edits near offset {edit.start}; previous edit ends at {previous_end}"
            )
        previous_end = edit.end

    result = text
    for edit in reversed(ordered):
        result = result[: edit.start] + edit.replacement + result[edit.end :]
    return result
