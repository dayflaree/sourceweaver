"""Encoding-aware VMF document wrapper."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from sourceweaver.vmf.parser import ParsedVmf, parse
from sourceweaver.vmf.patch import TextEdit, apply_edits


@dataclass(frozen=True, slots=True)
class VmfDocument:
    """A VMF document with exact original bytes and a lossless syntax tree."""

    path: Path | None
    raw_bytes: bytes
    encoding: str
    newline: str
    syntax: ParsedVmf

    @classmethod
    def from_bytes(cls, data: bytes, *, path: Path | None = None) -> VmfDocument:
        """Decode a VMF using BOM-aware UTF-8, then Windows-1251 fallback."""
        if data.startswith(b"\xef\xbb\xbf"):
            encoding = "utf-8-sig"
        else:
            try:
                data.decode("utf-8")
            except UnicodeDecodeError:
                encoding = "cp1251"
            else:
                encoding = "utf-8"
        text = data.decode(encoding)
        newline = "\r\n" if "\r\n" in text else "\n"
        return cls(
            path=path, raw_bytes=data, encoding=encoding, newline=newline, syntax=parse(text)
        )

    @classmethod
    def read(cls, path: str | Path) -> VmfDocument:
        """Read and parse a VMF from disk."""
        resolved = Path(path)
        return cls.from_bytes(resolved.read_bytes(), path=resolved)

    @property
    def text(self) -> str:
        return self.syntax.text

    def render_bytes(self) -> bytes:
        """Render without edits. This must equal the original bytes."""
        return self.syntax.render().encode(self.encoding)

    def patched_bytes(self, edits: tuple[TextEdit, ...] | list[TextEdit]) -> bytes:
        """Apply source edits while retaining the original encoding."""
        return apply_edits(self.text, edits).encode(self.encoding)
