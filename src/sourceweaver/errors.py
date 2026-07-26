"""Project exception hierarchy."""


class SourceWeaverError(Exception):
    """Base exception for user-facing SourceWeaver failures."""


class VmfSyntaxError(SourceWeaverError):
    """Raised when a VMF/KeyValues document is structurally invalid."""

    def __init__(self, message: str, *, offset: int, line: int, column: int) -> None:
        self.offset = offset
        self.line = line
        self.column = column
        super().__init__(f"{message} at line {line}, column {column}")


class PatchConflictError(SourceWeaverError):
    """Raised when two source-preserving edits overlap."""


class ProfileError(SourceWeaverError):
    """Raised when a game/compiler profile is invalid or incomplete."""
