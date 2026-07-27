"""Encoding-aware VMF document wrapper."""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Final

from sourceweaver.errors import ArtifactChangedError, VmfLimitError
from sourceweaver.fileio import stable_stat_identity
from sourceweaver.vmf.lexer import DEFAULT_MAX_SOURCE_CHARS, DEFAULT_MAX_TOKENS
from sourceweaver.vmf.parser import DEFAULT_MAX_DEPTH, ParsedVmf, parse
from sourceweaver.vmf.patch import TextEdit, apply_edits

DEFAULT_MAX_SOURCE_BYTES: Final[int] = 256 * 1024 * 1024


def _detect_newlines(text: str) -> tuple[str, str]:
    """Return the preferred newline sequence and an exact style label."""
    crlf = text.count("\r\n")
    without_crlf = text.replace("\r\n", "")
    lf = without_crlf.count("\n")
    cr = without_crlf.count("\r")
    present = [("\r\n", "CRLF", crlf), ("\n", "LF", lf), ("\r", "CR", cr)]
    used = [entry for entry in present if entry[2] > 0]
    if not used:
        return "\n", "NONE"
    preferred, label, _ = max(used, key=lambda entry: entry[2])
    if len(used) > 1:
        return preferred, "MIXED"
    return preferred, label


@dataclass(frozen=True, slots=True)
class VmfDocument:
    """A VMF document with exact original bytes and a lossless syntax tree."""

    path: Path | None
    raw_bytes: bytes
    encoding: str
    newline: str
    newline_style: str
    syntax: ParsedVmf

    @classmethod
    def from_bytes(
        cls,
        data: bytes,
        *,
        path: Path | None = None,
        max_bytes: int = DEFAULT_MAX_SOURCE_BYTES,
        max_chars: int = DEFAULT_MAX_SOURCE_CHARS,
        max_tokens: int = DEFAULT_MAX_TOKENS,
        max_depth: int = DEFAULT_MAX_DEPTH,
    ) -> VmfDocument:
        """Decode a VMF using BOM-aware UTF-8, then Windows-1251 fallback."""
        if max_bytes < 1:
            raise ValueError("VMF byte limit must be positive")
        if len(data) > max_bytes:
            raise VmfLimitError(f"VMF is {len(data):,} bytes; configured limit is {max_bytes:,}")

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
        newline, newline_style = _detect_newlines(text)
        return cls(
            path=path,
            raw_bytes=data,
            encoding=encoding,
            newline=newline,
            newline_style=newline_style,
            syntax=parse(
                text,
                max_chars=max_chars,
                max_tokens=max_tokens,
                max_depth=max_depth,
            ),
        )

    @classmethod
    def read(
        cls,
        path: str | Path,
        *,
        max_bytes: int = DEFAULT_MAX_SOURCE_BYTES,
        max_chars: int = DEFAULT_MAX_SOURCE_CHARS,
        max_tokens: int = DEFAULT_MAX_TOKENS,
        max_depth: int = DEFAULT_MAX_DEPTH,
    ) -> VmfDocument:
        """Read one stable, bounded VMF from disk and parse it losslessly."""
        source = Path(path)
        with source.open("rb") as stream:
            before = os.fstat(stream.fileno())
            data = stream.read(max_bytes + 1)
            after = os.fstat(stream.fileno())
        try:
            current = source.stat()
        except OSError as exc:
            raise ArtifactChangedError(f"VMF path disappeared while being read: {source}") from exc

        identity_before = stable_stat_identity(before)
        identity_after = stable_stat_identity(after)
        identity_current = stable_stat_identity(current)
        if identity_before != identity_after or identity_before != identity_current:
            raise ArtifactChangedError(f"VMF changed while being read: {source}")
        if len(data) > max_bytes:
            raise VmfLimitError(f"VMF exceeds configured byte limit of {max_bytes:,}: {source}")
        return cls.from_bytes(
            data,
            path=source,
            max_bytes=max_bytes,
            max_chars=max_chars,
            max_tokens=max_tokens,
            max_depth=max_depth,
        )

    @property
    def text(self) -> str:
        return self.syntax.text

    def render_bytes(self) -> bytes:
        """Render without edits. This must equal the original bytes."""
        return self.syntax.render().encode(self.encoding)

    def patched_bytes(self, edits: tuple[TextEdit, ...] | list[TextEdit]) -> bytes:
        """Apply source edits while retaining the original encoding."""
        return apply_edits(self.text, edits).encode(self.encoding)
