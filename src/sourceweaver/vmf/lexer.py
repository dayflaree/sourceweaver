"""A lossless lexer for VMF's KeyValues-derived syntax.

The lexer retains every byte represented as decoded text: whitespace, comments,
quoted strings, bare atoms, and braces. VMF files are normally Windows-1251 or
ASCII-compatible text. Byte decoding is handled by :mod:`document`.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from typing import Final

from sourceweaver.errors import VmfSyntaxError


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
    """One source token with an exact span."""

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
                    # Source KeyValues accepts game-dependent escape behavior.
                    # Preserve unknown escapes semantically as two characters.
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
    newline_count = raw.count("\n")
    if newline_count == 0:
        return line, column + len(raw)
    return line + newline_count, len(raw.rsplit("\n", 1)[-1]) + 1


def lex(text: str) -> tuple[Token, ...]:
    """Lex *text* while preserving all source characters and spans."""
    tokens: list[Token] = []
    index = 0
    line = 1
    column = 1
    length = len(text)

    def emit(kind: TokenKind, start: int, end: int, token_line: int, token_col: int) -> None:
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
