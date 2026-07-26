"""Lossless concrete-syntax parser for VMF/KeyValues documents."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TypeAlias

from sourceweaver.errors import VmfSyntaxError
from sourceweaver.vmf.lexer import TRIVIA_KINDS, Token, TokenKind, lex


@dataclass(frozen=True, slots=True)
class PairNode:
    """A key/value entry retaining original source tokens."""

    key: Token
    value: Token
    start: int
    end: int


@dataclass(frozen=True, slots=True)
class BlockNode:
    """A named KeyValues block and its recursively parsed entries."""

    key: Token
    open_brace: Token
    entries: tuple[EntryNode, ...]
    close_brace: Token
    start: int
    end: int


EntryNode: TypeAlias = PairNode | BlockNode


@dataclass(frozen=True, slots=True)
class ParsedVmf:
    """Parsed syntax tree plus the exact token stream."""

    text: str
    tokens: tuple[Token, ...]
    entries: tuple[EntryNode, ...]

    def render(self) -> str:
        """Return the original text byte-for-text at the decoded-text layer."""
        return "".join(token.raw for token in self.tokens if token.kind is not TokenKind.EOF)

    def blocks(self, name: str | None = None) -> tuple[BlockNode, ...]:
        """Return top-level blocks, optionally filtered case-insensitively."""
        blocks = tuple(entry for entry in self.entries if isinstance(entry, BlockNode))
        if name is None:
            return blocks
        folded = name.casefold()
        return tuple(block for block in blocks if block.key.value.casefold() == folded)


class _Parser:
    def __init__(self, text: str, tokens: tuple[Token, ...]) -> None:
        self.text = text
        self.tokens = tokens
        self.index = 0

    def _skip_trivia(self) -> None:
        while self.tokens[self.index].kind in TRIVIA_KINDS:
            self.index += 1

    def _current(self) -> Token:
        return self.tokens[self.index]

    @staticmethod
    def _is_value_token(token: Token) -> bool:
        return token.kind in {TokenKind.STRING, TokenKind.ATOM}

    def parse_entries(self, *, stop_on_rbrace: bool) -> tuple[EntryNode, ...]:
        entries: list[EntryNode] = []
        while True:
            self._skip_trivia()
            current = self._current()
            if current.kind is TokenKind.EOF:
                if stop_on_rbrace:
                    raise VmfSyntaxError(
                        "Missing closing brace",
                        offset=current.start,
                        line=current.line,
                        column=current.column,
                    )
                return tuple(entries)
            if current.kind is TokenKind.RBRACE:
                if stop_on_rbrace:
                    return tuple(entries)
                raise VmfSyntaxError(
                    "Unexpected closing brace",
                    offset=current.start,
                    line=current.line,
                    column=current.column,
                )
            if not self._is_value_token(current):
                raise VmfSyntaxError(
                    f"Expected key, found {current.kind.value}",
                    offset=current.start,
                    line=current.line,
                    column=current.column,
                )

            key = current
            self.index += 1
            self._skip_trivia()
            next_token = self._current()

            if next_token.kind is TokenKind.LBRACE:
                open_brace = next_token
                self.index += 1
                children = self.parse_entries(stop_on_rbrace=True)
                close_brace = self._current()
                self.index += 1
                entries.append(
                    BlockNode(
                        key=key,
                        open_brace=open_brace,
                        entries=children,
                        close_brace=close_brace,
                        start=key.start,
                        end=close_brace.end,
                    )
                )
                continue

            if self._is_value_token(next_token):
                self.index += 1
                entries.append(
                    PairNode(key=key, value=next_token, start=key.start, end=next_token.end)
                )
                continue

            raise VmfSyntaxError(
                "Expected a value or opening brace after key",
                offset=next_token.start,
                line=next_token.line,
                column=next_token.column,
            )


def parse(text: str) -> ParsedVmf:
    """Parse VMF text into a lossless concrete syntax tree."""
    tokens = lex(text)
    parser = _Parser(text, tokens)
    entries = parser.parse_entries(stop_on_rbrace=False)
    return ParsedVmf(text=text, tokens=tokens, entries=entries)
