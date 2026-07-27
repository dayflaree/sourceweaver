"""A lossless lexer for VMF's KeyValues-derived syntax.

The lexer retains every decoded source character: whitespace, comments, quoted
strings, bare atoms, and braces. Byte decoding is handled by :mod:`document`.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from typing import Final

from sourceweaver.errors import VmfLimitError, VmfSyntaxError

DEFAULT_MAX_SOURCE_CHARS: Final[int] = 256 * 1024 * 1024
DEFAULT_MAX_TOKENS: Final[int] = 10_000_000


class TokenKind(StrEnum):
    """Token categories used by the lossless parser."""

    WHITESPACE = "whitespace"
    COMMENT = "comment"
    STRING = "string"
    ATOM = "atom"
    LBRACE = "lbrace"
    RBRACE = "rbrace"
    EOF = "eof"


TRIVIA_KINDS: Final[frozenset[TokenKind]] = frozenset({TokenKind.WHITESPACE, TokenKind.COMMENT})


@dataclass(frozen=True, slots=True)
class Token:
    """One source token with an exact character span."""

    kind: TokenKind
    raw: str
    start: int
    end: int
    line: int
    column: int

    @property
    def value(self) -> str:
        """Return an interpreted string value without modifying source text."""
        if self.kind is not TokenKind.STRING:
            return self.raw
        body = self.raw[1:-1]
        output: list[str] = []
        escaped = False
        for char in body:
            if escaped:
                if char in {'"', "\\"}:
                    output.append(char)
                else:
                    # Source branches differ in escape handling. Preserve unknown
                    # escapes semantically as the original two characters.
                    output.extend(("\\", char))
                escaped = False
            elif char == "\\":
                escaped = True
            else:
                output.append(char)
        if escaped:
            output.append("\\")
        return "".join(output)


def _advance_position(raw: str, line: int, column: int) -> tuple[int, int]:
    normalized = raw.replace("\r\n", "\n").replace("\r", "\n")
    newline_count = normalized.count("\n")
    if newline_count == 0:
        return line, column + len(raw)
    return line + newline_count, len(normalized.rsplit("\n", 1)[-1]) + 1


def lex(
    text: str,
    *,
    max_chars: int = DEFAULT_MAX_SOURCE_CHARS,
    max_tokens: int = DEFAULT_MAX_TOKENS,
) -> tuple[Token, ...]:
    """Lex *text* while preserving all source characters and spans.

    Limits prevent untrusted files from causing unbounded memory consumption.
    Callers may lower them for services with stricter resource budgets.
    """
    if max_chars < 0 or max_tokens < 1:
        raise ValueError("Lexer limits must be positive")
    if len(text) > max_chars:
        raise VmfLimitError(
            f"VMF contains {len(text):,} decoded characters; limit is {max_chars:,}"
        )

    tokens: list[Token] = []
    index = 0
    line = 1
    column = 1
    length = len(text)

    def emit(kind: TokenKind, start: int, end: int, token_line: int, token_col: int) -> None:
        if len(tokens) >= max_tokens:
            raise VmfLimitError(f"VMF token count exceeds configured limit of {max_tokens:,}")
        tokens.append(Token(kind, text[start:end], start, end, token_line, token_col))

    while index < length:
        start = index
        token_line, token_col = line, column
        char = text[index]

        if char.isspace():
            index += 1
            while index < length and text[index].isspace():
                index += 1
            raw = text[start:index]
            emit(TokenKind.WHITESPACE, start, index, token_line, token_col)
            line, column = _advance_position(raw, line, column)
            continue

        if char == "/" and index + 1 < length and text[index + 1] == "/":
            index += 2
            while index < length and text[index] not in "\r\n":
                index += 1
            raw = text[start:index]
            emit(TokenKind.COMMENT, start, index, token_line, token_col)
            line, column = _advance_position(raw, line, column)
            continue

        if char == '"':
            index += 1
            escaped = False
            while index < length:
                current = text[index]
                index += 1
                if escaped:
                    escaped = False
                elif current == "\\":
                    escaped = True
                elif current == '"':
                    break
            else:
                raise VmfSyntaxError(
                    "Unterminated quoted string",
                    offset=start,
                    line=token_line,
                    column=token_col,
                )
            raw = text[start:index]
            emit(TokenKind.STRING, start, index, token_line, token_col)
            line, column = _advance_position(raw, line, column)
            continue

        if char == "{":
            index += 1
            emit(TokenKind.LBRACE, start, index, token_line, token_col)
            column += 1
            continue

        if char == "}":
            index += 1
            emit(TokenKind.RBRACE, start, index, token_line, token_col)
            column += 1
            continue

        index += 1
        while index < length:
            current = text[index]
            if current.isspace() or current in '{}"':
                break
            if current == "/" and index + 1 < length and text[index + 1] == "/":
                break
            index += 1
        raw = text[start:index]
        emit(TokenKind.ATOM, start, index, token_line, token_col)
        line, column = _advance_position(raw, line, column)

    tokens.append(Token(TokenKind.EOF, "", length, length, line, column))
    return tuple(tokens)
