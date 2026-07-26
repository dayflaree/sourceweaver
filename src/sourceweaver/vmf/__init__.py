"""Lossless VMF syntax and patching primitives."""

from sourceweaver.vmf.document import VmfDocument
from sourceweaver.vmf.lexer import Token, TokenKind, lex
from sourceweaver.vmf.parser import BlockNode, EntryNode, PairNode, parse
from sourceweaver.vmf.patch import TextEdit, apply_edits

__all__ = [
    "BlockNode",
    "EntryNode",
    "PairNode",
    "TextEdit",
    "Token",
    "TokenKind",
    "VmfDocument",
    "apply_edits",
    "lex",
    "parse",
]
