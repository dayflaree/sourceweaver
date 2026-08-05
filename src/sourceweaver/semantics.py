"""CST-backed semantic extraction primitives.

This module is intentionally read-only. It classifies only data that can be
anchored to unique CST spans and does not authorize source rewrites.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum

from sourceweaver.vmf.document import VmfDocument
from sourceweaver.vmf.parser import BlockNode, PairNode


class EntityBlockKind(StrEnum):
    """Top-level VMF block kinds represented as semantic entities."""

    WORLD = "world"
    ENTITY = "entity"


class ReferenceKind(StrEnum):
    """Supported target-reference sources."""

    PARENT = "parentname"
    OUTPUT = "output"
    SPECIAL = "special"
    WILDCARD = "wildcard"


@dataclass(frozen=True, order=True, slots=True)
class SourceSpan:
    """Half-open character span anchored to the decoded VMF text."""

    start: int
    end: int
    line: int
    column: int


@dataclass(frozen=True, slots=True)
class SemanticPair:
    """A key/value pair with exact CST provenance."""

    key: str
    value: str
    key_span: SourceSpan
    value_span: SourceSpan
    entry_span: SourceSpan
    ordinal: int


@dataclass(frozen=True, slots=True)
class OutputConnection:
    """Parsed entity output value with original pair provenance retained."""

    event: str
    raw_value: str
    target: str
    input_name: str
    parameter: str
    delay: str
    fire_count: str
    separator: str
    pair: SemanticPair
    well_formed: bool


@dataclass(frozen=True, slots=True)
class SemanticEntity:
    """A world or entity block reduced to ordered semantic key/value records."""

    index: int
    kind: EntityBlockKind
    block_span: SourceSpan
    keyvalues: tuple[SemanticPair, ...]
    classname: str | None
    hammer_id: str | None
    targetnames: tuple[SemanticPair, ...]
    outputs: tuple[OutputConnection, ...]


@dataclass(frozen=True, slots=True)
class TargetNameDefinition:
    """A targetname definition retained even when duplicates exist."""

    entity_index: int
    name: str
    pair: SemanticPair


@dataclass(frozen=True, slots=True)
class TargetNameReference:
    """A targetname reference with source provenance."""

    entity_index: int
    name: str
    kind: ReferenceKind
    pair: SemanticPair


@dataclass(frozen=True, slots=True)
class TargetNameGraph:
    """Resolved and unresolved targetname references for the read-only slice."""

    definitions: tuple[TargetNameDefinition, ...]
    references: tuple[TargetNameReference, ...]
    resolved_references: tuple[TargetNameReference, ...]
    unresolved_references: tuple[TargetNameReference, ...]
    ambiguous_references: tuple[TargetNameReference, ...]


@dataclass(frozen=True, slots=True)
class SemanticDocument:
    """Read-only semantic view over a lossless VMF document."""

    entities: tuple[SemanticEntity, ...]
    target_graph: TargetNameGraph


def _token_span(start: int, end: int, line: int, column: int) -> SourceSpan:
    return SourceSpan(start=start, end=end, line=line, column=column)


def _pair_from_node(node: PairNode, ordinal: int) -> SemanticPair:
    return SemanticPair(
        key=node.key.value,
        value=node.value.value,
        key_span=_token_span(node.key.start, node.key.end, node.key.line, node.key.column),
        value_span=_token_span(
            node.value.start,
            node.value.end,
            node.value.line,
            node.value.column,
        ),
        entry_span=_token_span(node.start, node.end, node.key.line, node.key.column),
        ordinal=ordinal,
    )


def _block_span(block: BlockNode) -> SourceSpan:
    return _token_span(block.start, block.end, block.key.line, block.key.column)


def _first_value(pairs: tuple[SemanticPair, ...], key: str) -> str | None:
    folded = key.casefold()
    for pair in pairs:
        if pair.key.casefold() == folded:
            return pair.value
    return None


def _split_output_value(value: str) -> tuple[str, tuple[str, ...]]:
    if "\x1b" in value:
        return "esc", tuple(value.split("\x1b"))

    fields: list[str] = []
    current: list[str] = []
    index = 0
    while index < len(value):
        char = value[index]
        if char == "\\" and index + 1 < len(value) and value[index + 1] == ",":
            current.append(",")
            index += 2
            continue
        if char == ",":
            fields.append("".join(current))
            current = []
            index += 1
            continue
        current.append(char)
        index += 1
    fields.append("".join(current))
    return "comma", tuple(fields)


def _parse_output(pair: SemanticPair) -> OutputConnection | None:
    if not pair.key.casefold().startswith("on"):
        return None

    separator, fields = _split_output_value(pair.value)
    padded = (*fields, "", "", "", "", "")
    target, input_name, parameter, delay, fire_count = padded[:5]
    return OutputConnection(
        event=pair.key,
        raw_value=pair.value,
        target=target,
        input_name=input_name,
        parameter=parameter,
        delay=delay,
        fire_count=fire_count,
        separator=separator,
        pair=pair,
        well_formed=len(fields) >= 2,
    )


def _is_special_reference(name: str) -> bool:
    return name.startswith("!")


def _is_wildcard_reference(name: str) -> bool:
    return "*" in name or "?" in name


def _reference_kind(name: str, default: ReferenceKind) -> ReferenceKind:
    if _is_special_reference(name):
        return ReferenceKind.SPECIAL
    if _is_wildcard_reference(name):
        return ReferenceKind.WILDCARD
    return default


def _entity_from_block(block: BlockNode, index: int) -> SemanticEntity:
    kind = EntityBlockKind(block.key.value.casefold())
    pairs = tuple(
        _pair_from_node(entry, ordinal)
        for ordinal, entry in enumerate(block.entries)
        if isinstance(entry, PairNode)
    )
    outputs = tuple(output for pair in pairs if (output := _parse_output(pair)) is not None)
    return SemanticEntity(
        index=index,
        kind=kind,
        block_span=_block_span(block),
        keyvalues=pairs,
        classname=_first_value(pairs, "classname"),
        hammer_id=_first_value(pairs, "id"),
        targetnames=tuple(pair for pair in pairs if pair.key.casefold() == "targetname"),
        outputs=outputs,
    )


def _build_target_graph(entities: tuple[SemanticEntity, ...]) -> TargetNameGraph:
    definitions: list[TargetNameDefinition] = []
    references: list[TargetNameReference] = []

    for entity in entities:
        for pair in entity.targetnames:
            if pair.value:
                definitions.append(
                    TargetNameDefinition(entity_index=entity.index, name=pair.value, pair=pair)
                )
        for pair in entity.keyvalues:
            if pair.key.casefold() == "parentname" and pair.value:
                references.append(
                    TargetNameReference(
                        entity_index=entity.index,
                        name=pair.value,
                        kind=_reference_kind(pair.value, ReferenceKind.PARENT),
                        pair=pair,
                    )
                )
        for output in entity.outputs:
            if output.target:
                references.append(
                    TargetNameReference(
                        entity_index=entity.index,
                        name=output.target,
                        kind=_reference_kind(output.target, ReferenceKind.OUTPUT),
                        pair=output.pair,
                    )
                )

    definitions_by_name: dict[str, list[TargetNameDefinition]] = {}
    for definition in definitions:
        definitions_by_name.setdefault(definition.name.casefold(), []).append(definition)

    resolved: list[TargetNameReference] = []
    unresolved: list[TargetNameReference] = []
    ambiguous: list[TargetNameReference] = []
    for reference in references:
        if reference.kind in {ReferenceKind.SPECIAL, ReferenceKind.WILDCARD}:
            continue
        matches = definitions_by_name.get(reference.name.casefold(), [])
        if len(matches) == 1:
            resolved.append(reference)
        elif len(matches) == 0:
            unresolved.append(reference)
        else:
            ambiguous.append(reference)

    return TargetNameGraph(
        definitions=tuple(definitions),
        references=tuple(references),
        resolved_references=tuple(resolved),
        unresolved_references=tuple(unresolved),
        ambiguous_references=tuple(ambiguous),
    )


def build_semantic_document(document: VmfDocument) -> SemanticDocument:
    """Build a read-only semantic view backed by the document CST.

    This function does not parse or rewrite FGD-typed data. It is the first
    supportable slice: entity blocks, direct keyvalues, targetname definitions,
    parentname references, and output-target references with exact source spans.
    """
    entity_blocks = [
        block
        for block in document.syntax.blocks()
        if block.key.value.casefold() in {EntityBlockKind.WORLD.value, EntityBlockKind.ENTITY.value}
    ]
    entities = tuple(_entity_from_block(block, index) for index, block in enumerate(entity_blocks))
    return SemanticDocument(entities=entities, target_graph=_build_target_graph(entities))
