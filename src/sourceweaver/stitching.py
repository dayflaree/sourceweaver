"""Read-only Source map transition graph extraction.

This module extracts transition facts from CST-backed semantic entities. It is
analysis-only and does not authorize alignment, stitching, VMF mutation, or
transition-volume deletion.
"""

from __future__ import annotations

import math
from collections.abc import Iterable
from dataclasses import dataclass
from enum import StrEnum

from sourceweaver.geometry import (
    BoundsRelation,
    BrushRelation,
    BrushSpatialRecord,
    GeometryTransformStatus,
    Vec3,
    classify_brush_relation,
    find_potential_brush_intersections,
    translate_convex_brush_for_analysis,
)
from sourceweaver.semantics import SemanticDocument, SemanticEntity, SemanticPair

_AMBIGUOUS_LANDMARK_MESSAGE = (
    "More than one info_landmark matches the trigger_changelevel landmark name."
)


class TransitionBlockerCode(StrEnum):
    """Deterministic blockers for transition graph authority."""

    CHANGELEVEL_MISSING_MAP = "changelevel_missing_map"
    CHANGELEVEL_MISSING_LANDMARK = "changelevel_missing_landmark"
    LANDMARK_NOT_FOUND = "landmark_not_found"
    LANDMARK_AMBIGUOUS = "landmark_ambiguous"
    LANDMARK_INVALID_ORIGIN = "landmark_invalid_origin"


class AlignmentStatus(StrEnum):
    """Final status for a read-only alignment hypothesis."""

    VALID = "valid"
    BLOCKED = "blocked"


class AlignmentBlockerCode(StrEnum):
    """Deterministic blockers for translation-only alignment authority."""

    SOURCE_EDGE_COUNT_UNSUPPORTED = "source_edge_count_unsupported"
    CANDIDATE_EDGE_COUNT_UNSUPPORTED = "candidate_edge_count_unsupported"
    SOURCE_EDGE_BLOCKED = "source_edge_blocked"
    CANDIDATE_EDGE_BLOCKED = "candidate_edge_blocked"
    LANDMARK_NAME_MISMATCH = "landmark_name_mismatch"
    LANDMARK_ORIGIN_UNAVAILABLE = "landmark_origin_unavailable"
    TRANSLATION_NONFINITE = "translation_nonfinite"


class SeamEvidenceStatus(StrEnum):
    """Final status for read-only seam overlap evidence."""

    VALID = "valid"
    BLOCKED = "blocked"


class SeamEvidenceBlockerCode(StrEnum):
    """Deterministic blockers for seam overlap evidence."""

    ALIGNMENT_BLOCKED = "alignment_blocked"
    CANDIDATE_TRANSFORM_BLOCKED = "candidate_transform_blocked"


class SeamDeletionEvidenceStatus(StrEnum):
    """Final status for review-only seam deletion evidence."""

    VALID = "valid"
    BLOCKED = "blocked"


class SeamDeletionClass(StrEnum):
    """Review-only deletion classes derived from exact brush relations."""

    CANDIDATE_EQUAL_VOLUME_DUPLICATE = "candidate_equal_volume_duplicate"
    CANDIDATE_CONTAINED_IN_SOURCE = "candidate_contained_in_source"
    SOURCE_CONTAINED_IN_CANDIDATE = "source_contained_in_candidate"
    PRESERVE_TOUCHING_SEAM = "preserve_touching_seam"
    PRESERVE_UNSAFE_OVERLAP = "preserve_unsafe_overlap"
    PRESERVE_DISJOINT_OR_UNCLASSIFIED = "preserve_disjoint_or_unclassified"


@dataclass(frozen=True, slots=True)
class TransitionBlocker:
    """A reason a transition edge cannot become stitching authority."""

    code: TransitionBlockerCode
    message: str
    entity_index: int | None = None


@dataclass(frozen=True, slots=True)
class LandmarkDefinition:
    """One `info_landmark` definition with source-backed fields."""

    entity_index: int
    hammer_id: str | None
    name: str
    origin: Vec3 | None
    targetname_pair: SemanticPair
    origin_pair: SemanticPair | None
    blockers: tuple[TransitionBlocker, ...]


@dataclass(frozen=True, slots=True)
class TransitionVolume:
    """One `trigger_transition` volume indexed by direct names."""

    entity_index: int
    hammer_id: str | None
    names: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class ChangeLevelTrigger:
    """One `trigger_changelevel` trigger with raw keyvalue provenance."""

    entity_index: int
    hammer_id: str | None
    destination_raw: str | None
    destination_pair: SemanticPair | None
    landmark_name: str | None
    landmark_pair: SemanticPair | None
    targetnames: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class TransitionEdge:
    """A read-only transition edge candidate and its evidence."""

    changelevel_entity_index: int
    changelevel_hammer_id: str | None
    destination_raw: str
    destination_normalized: str
    landmark_name: str
    landmark_matches: tuple[LandmarkDefinition, ...]
    landmark_origin: Vec3 | None
    transition_volume_matches: tuple[TransitionVolume, ...]
    blockers: tuple[TransitionBlocker, ...]


@dataclass(frozen=True, slots=True)
class TransitionGraph:
    """Read-only transition graph facts extracted from one semantic VMF."""

    changelevels: tuple[ChangeLevelTrigger, ...]
    landmarks: tuple[LandmarkDefinition, ...]
    transition_volumes: tuple[TransitionVolume, ...]
    edges: tuple[TransitionEdge, ...]


@dataclass(frozen=True, slots=True)
class AlignmentBlocker:
    """A reason a pair of transition graphs cannot be aligned safely."""

    code: AlignmentBlockerCode
    message: str
    source_entity_index: int | None = None
    candidate_entity_index: int | None = None


@dataclass(frozen=True, slots=True)
class TranslationAlignmentHypothesis:
    """Read-only translation hypothesis between two directly connected maps."""

    status: AlignmentStatus
    source_map_normalized: str
    candidate_map_normalized: str
    offset: Vec3 | None
    source_edge: TransitionEdge | None
    candidate_edge: TransitionEdge | None
    blockers: tuple[AlignmentBlocker, ...]
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class SeamEvidenceBlocker:
    """A reason seam overlap evidence cannot be built safely."""

    code: SeamEvidenceBlockerCode
    message: str
    record_key: str | None = None
    geometry_blocker_codes: tuple[str, ...] = ()


@dataclass(frozen=True, slots=True)
class SeamBrushPairEvidence:
    """One translated candidate/source brush relation near a seam."""

    source_key: str
    candidate_key: str
    bounds_relation: BoundsRelation
    brush_relation: BrushRelation


@dataclass(frozen=True, slots=True)
class SeamOverlapEvidence:
    """Read-only seam overlap evidence after applying an alignment offset."""

    status: SeamEvidenceStatus
    translated_candidate_records: tuple[BrushSpatialRecord, ...]
    brush_pairs: tuple[SeamBrushPairEvidence, ...]
    blockers: tuple[SeamEvidenceBlocker, ...]
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class SeamDeletionItemEvidence:
    """One review-only deletion class for a seam brush pair."""

    source_key: str
    candidate_key: str
    brush_relation: BrushRelation
    deletion_class: SeamDeletionClass
    remove_source: bool
    remove_candidate: bool


@dataclass(frozen=True, slots=True)
class SeamDeletionEvidence:
    """Review-only seam deletion evidence with no mutation authority."""

    status: SeamDeletionEvidenceStatus
    items: tuple[SeamDeletionItemEvidence, ...]
    blockers: tuple[SeamEvidenceBlocker, ...]
    mutation_authorized: bool = False


def normalize_map_name(raw: str) -> str:
    """Normalize a destination map string for matching only."""
    normalized = raw.strip().casefold()
    if normalized.endswith(".bsp"):
        return normalized[:-4]
    return normalized


def build_transition_graph(document: SemanticDocument) -> TransitionGraph:
    """Extract transition graph facts without making stitching decisions."""
    landmarks = tuple(
        _landmark_from_entity(entity)
        for entity in document.entities
        if _classname_is(entity, "info_landmark") and entity.targetnames
    )
    transition_volumes = tuple(
        _transition_volume_from_entity(entity)
        for entity in document.entities
        if _classname_is(entity, "trigger_transition")
    )
    changelevels = tuple(
        _changelevel_from_entity(entity)
        for entity in document.entities
        if _classname_is(entity, "trigger_changelevel")
    )
    return TransitionGraph(
        changelevels=changelevels,
        landmarks=landmarks,
        transition_volumes=transition_volumes,
        edges=tuple(
            _edge_from_changelevel(changelevel, landmarks, transition_volumes)
            for changelevel in changelevels
        ),
    )


def build_translation_alignment_hypothesis(
    source_graph: TransitionGraph,
    candidate_graph: TransitionGraph,
    *,
    source_map_name: str,
    candidate_map_name: str,
) -> TranslationAlignmentHypothesis:
    """Build the initial one-edge, translation-only alignment hypothesis.

    The offset moves candidate-map geometry into source-map coordinates using
    ``source_landmark_origin - candidate_landmark_origin``. The result is
    evidence for later seam planning and carries no VMF mutation authority.
    """
    source_map_normalized = normalize_map_name(source_map_name)
    candidate_map_normalized = normalize_map_name(candidate_map_name)
    source_edges = _edges_to_map(source_graph, candidate_map_normalized)
    candidate_edges = _edges_to_map(candidate_graph, source_map_normalized)
    blockers: list[AlignmentBlocker] = []

    if len(source_edges) != 1:
        blockers.append(
            AlignmentBlocker(
                code=AlignmentBlockerCode.SOURCE_EDGE_COUNT_UNSUPPORTED,
                message=(
                    "Expected exactly one source trigger_changelevel edge to the candidate map."
                ),
            )
        )
    if len(candidate_edges) != 1:
        blockers.append(
            AlignmentBlocker(
                code=AlignmentBlockerCode.CANDIDATE_EDGE_COUNT_UNSUPPORTED,
                message=(
                    "Expected exactly one candidate trigger_changelevel edge back "
                    "to the source map."
                ),
            )
        )
    source_edge = _only_edge(source_edges)
    candidate_edge = _only_edge(candidate_edges)
    if blockers or source_edge is None or candidate_edge is None:
        return _blocked_alignment(
            source_map_normalized=source_map_normalized,
            candidate_map_normalized=candidate_map_normalized,
            source_edge=source_edge,
            candidate_edge=candidate_edge,
            blockers=tuple(blockers),
        )

    if source_edge.blockers:
        blockers.append(
            AlignmentBlocker(
                code=AlignmentBlockerCode.SOURCE_EDGE_BLOCKED,
                message="Source transition edge has transition blockers.",
                source_entity_index=source_edge.changelevel_entity_index,
            )
        )
    if candidate_edge.blockers:
        blockers.append(
            AlignmentBlocker(
                code=AlignmentBlockerCode.CANDIDATE_EDGE_BLOCKED,
                message="Candidate transition edge has transition blockers.",
                candidate_entity_index=candidate_edge.changelevel_entity_index,
            )
        )
    if source_edge.landmark_name.casefold() != candidate_edge.landmark_name.casefold():
        blockers.append(
            AlignmentBlocker(
                code=AlignmentBlockerCode.LANDMARK_NAME_MISMATCH,
                message="Source and candidate transition edges use different landmarks.",
                source_entity_index=source_edge.changelevel_entity_index,
                candidate_entity_index=candidate_edge.changelevel_entity_index,
            )
        )
    if blockers:
        return _blocked_alignment(
            source_map_normalized=source_map_normalized,
            candidate_map_normalized=candidate_map_normalized,
            source_edge=source_edge,
            candidate_edge=candidate_edge,
            blockers=tuple(blockers),
        )

    source_origin = source_edge.landmark_origin
    candidate_origin = candidate_edge.landmark_origin
    if source_origin is None or candidate_origin is None:
        blockers.append(
            AlignmentBlocker(
                code=AlignmentBlockerCode.LANDMARK_ORIGIN_UNAVAILABLE,
                message="A matched transition edge lacks a validated landmark origin.",
                source_entity_index=source_edge.changelevel_entity_index,
                candidate_entity_index=candidate_edge.changelevel_entity_index,
            )
        )
        return _blocked_alignment(
            source_map_normalized=source_map_normalized,
            candidate_map_normalized=candidate_map_normalized,
            source_edge=source_edge,
            candidate_edge=candidate_edge,
            blockers=tuple(blockers),
        )

    offset = source_origin - candidate_origin
    if not _is_finite_vec(offset):
        return _blocked_alignment(
            source_map_normalized=source_map_normalized,
            candidate_map_normalized=candidate_map_normalized,
            source_edge=source_edge,
            candidate_edge=candidate_edge,
            blockers=(
                AlignmentBlocker(
                    code=AlignmentBlockerCode.TRANSLATION_NONFINITE,
                    message="Computed landmark translation has a non-finite coordinate.",
                    source_entity_index=source_edge.changelevel_entity_index,
                    candidate_entity_index=candidate_edge.changelevel_entity_index,
                ),
            ),
        )
    return TranslationAlignmentHypothesis(
        status=AlignmentStatus.VALID,
        source_map_normalized=source_map_normalized,
        candidate_map_normalized=candidate_map_normalized,
        offset=offset,
        source_edge=source_edge,
        candidate_edge=candidate_edge,
        blockers=(),
    )


def build_seam_overlap_evidence(
    alignment: TranslationAlignmentHypothesis,
    source_records: Iterable[BrushSpatialRecord],
    candidate_records: Iterable[BrushSpatialRecord],
    *,
    expansion: float = 0.0,
) -> SeamOverlapEvidence:
    """Translate candidate brushes and classify source/candidate seam relations.

    This is a read-only evidence builder. It transforms candidate brush records
    in memory, runs deterministic AABB broad-phase filtering, then attaches exact
    convex brush relation classifications for review and later seam planning.
    """
    if alignment.status is not AlignmentStatus.VALID or alignment.offset is None:
        return SeamOverlapEvidence(
            status=SeamEvidenceStatus.BLOCKED,
            translated_candidate_records=(),
            brush_pairs=(),
            blockers=(
                SeamEvidenceBlocker(
                    code=SeamEvidenceBlockerCode.ALIGNMENT_BLOCKED,
                    message="Alignment hypothesis is blocked or lacks an offset.",
                ),
            ),
        )

    transformed_records: list[BrushSpatialRecord] = []
    blockers: list[SeamEvidenceBlocker] = []
    for record in candidate_records:
        transform = translate_convex_brush_for_analysis(record.brush, alignment.offset)
        if transform.status is not GeometryTransformStatus.VALID or transform.brush is None:
            blockers.append(
                SeamEvidenceBlocker(
                    code=SeamEvidenceBlockerCode.CANDIDATE_TRANSFORM_BLOCKED,
                    message="Candidate brush could not be translated for seam evidence.",
                    record_key=record.key,
                    geometry_blocker_codes=tuple(blocker.code for blocker in transform.blockers),
                )
            )
            continue
        transformed_records.append(BrushSpatialRecord(record.key, transform.brush))

    translated = tuple(transformed_records)
    if blockers:
        return SeamOverlapEvidence(
            status=SeamEvidenceStatus.BLOCKED,
            translated_candidate_records=translated,
            brush_pairs=(),
            blockers=tuple(blockers),
        )

    sources = tuple(source_records)
    source_by_key = {record.key: record.brush for record in sources}
    translated_by_key = {record.key: record.brush for record in translated}
    brush_pairs = tuple(
        SeamBrushPairEvidence(
            source_key=candidate.a_key,
            candidate_key=candidate.b_key,
            bounds_relation=candidate.bounds_relation,
            brush_relation=classify_brush_relation(
                source_by_key[candidate.a_key], translated_by_key[candidate.b_key]
            ),
        )
        for candidate in find_potential_brush_intersections(
            sources, translated, expansion=expansion
        )
    )
    return SeamOverlapEvidence(
        status=SeamEvidenceStatus.VALID,
        translated_candidate_records=translated,
        brush_pairs=brush_pairs,
        blockers=(),
    )


def build_seam_deletion_evidence(
    seam_evidence: SeamOverlapEvidence,
) -> SeamDeletionEvidence:
    """Classify seam brush pairs as review-only deletion evidence.

    These classes identify pairs that later phases may evaluate. They do not
    authorize VMF deletion or output changes.
    """
    if seam_evidence.status is not SeamEvidenceStatus.VALID:
        return SeamDeletionEvidence(
            status=SeamDeletionEvidenceStatus.BLOCKED,
            items=(),
            blockers=seam_evidence.blockers,
        )
    return SeamDeletionEvidence(
        status=SeamDeletionEvidenceStatus.VALID,
        items=tuple(_deletion_item_from_pair(pair) for pair in seam_evidence.brush_pairs),
        blockers=(),
    )


def _classname_is(entity: SemanticEntity, classname: str) -> bool:
    return entity.classname is not None and entity.classname.casefold() == classname


def _edges_to_map(graph: TransitionGraph, normalized_map_name: str) -> tuple[TransitionEdge, ...]:
    return tuple(edge for edge in graph.edges if edge.destination_normalized == normalized_map_name)


def _only_edge(edges: tuple[TransitionEdge, ...]) -> TransitionEdge | None:
    if len(edges) == 1:
        return next(iter(edges))
    return None


def _blocked_alignment(
    *,
    source_map_normalized: str,
    candidate_map_normalized: str,
    source_edge: TransitionEdge | None,
    candidate_edge: TransitionEdge | None,
    blockers: tuple[AlignmentBlocker, ...],
) -> TranslationAlignmentHypothesis:
    return TranslationAlignmentHypothesis(
        status=AlignmentStatus.BLOCKED,
        source_map_normalized=source_map_normalized,
        candidate_map_normalized=candidate_map_normalized,
        offset=None,
        source_edge=source_edge,
        candidate_edge=candidate_edge,
        blockers=blockers,
    )


def _is_finite_vec(point: Vec3) -> bool:
    return all(math.isfinite(value) for value in point.as_tuple())


def _deletion_item_from_pair(pair: SeamBrushPairEvidence) -> SeamDeletionItemEvidence:
    deletion_class, remove_source, remove_candidate = _classify_deletion(pair.brush_relation)
    return SeamDeletionItemEvidence(
        source_key=pair.source_key,
        candidate_key=pair.candidate_key,
        brush_relation=pair.brush_relation,
        deletion_class=deletion_class,
        remove_source=remove_source,
        remove_candidate=remove_candidate,
    )


def _classify_deletion(relation: BrushRelation) -> tuple[SeamDeletionClass, bool, bool]:
    if relation is BrushRelation.EQUAL_VOLUME:
        return SeamDeletionClass.CANDIDATE_EQUAL_VOLUME_DUPLICATE, False, True
    if relation is BrushRelation.A_CONTAINS_B:
        return SeamDeletionClass.CANDIDATE_CONTAINED_IN_SOURCE, False, True
    if relation is BrushRelation.B_CONTAINS_A:
        return SeamDeletionClass.SOURCE_CONTAINED_IN_CANDIDATE, True, False
    if relation is BrushRelation.TOUCHING:
        return SeamDeletionClass.PRESERVE_TOUCHING_SEAM, False, False
    if relation is BrushRelation.OVERLAPPING:
        return SeamDeletionClass.PRESERVE_UNSAFE_OVERLAP, False, False
    return SeamDeletionClass.PRESERVE_DISJOINT_OR_UNCLASSIFIED, False, False


def _first_pair(entity: SemanticEntity, key: str) -> SemanticPair | None:
    folded = key.casefold()
    for pair in entity.keyvalues:
        if pair.key.casefold() == folded:
            return pair
    return None


def _direct_values(entity: SemanticEntity, key: str) -> tuple[str, ...]:
    folded = key.casefold()
    return tuple(
        pair.value for pair in entity.keyvalues if pair.key.casefold() == folded and pair.value
    )


def _parse_origin(pair: SemanticPair | None) -> tuple[Vec3 | None, tuple[TransitionBlocker, ...]]:
    if pair is None:
        return None, (
            TransitionBlocker(
                code=TransitionBlockerCode.LANDMARK_INVALID_ORIGIN,
                message="info_landmark is missing a direct origin keyvalue.",
            ),
        )
    parts = pair.value.split()
    if len(parts) != 3:
        return None, (
            TransitionBlocker(
                code=TransitionBlockerCode.LANDMARK_INVALID_ORIGIN,
                message="info_landmark origin must contain exactly three numeric coordinates.",
            ),
        )
    try:
        origin = Vec3(*(float(part) for part in parts))
    except ValueError:
        return None, (
            TransitionBlocker(
                code=TransitionBlockerCode.LANDMARK_INVALID_ORIGIN,
                message="info_landmark origin contains a non-numeric coordinate.",
            ),
        )
    if not all(math.isfinite(value) for value in origin.as_tuple()):
        return None, (
            TransitionBlocker(
                code=TransitionBlockerCode.LANDMARK_INVALID_ORIGIN,
                message="info_landmark origin contains a non-finite coordinate.",
            ),
        )
    return origin, ()


def _landmark_from_entity(entity: SemanticEntity) -> LandmarkDefinition:
    targetname_pair = entity.targetnames[0]
    origin_pair = _first_pair(entity, "origin")
    origin, blockers = _parse_origin(origin_pair)
    blockers = tuple(
        TransitionBlocker(code=blocker.code, message=blocker.message, entity_index=entity.index)
        for blocker in blockers
    )
    return LandmarkDefinition(
        entity_index=entity.index,
        hammer_id=entity.hammer_id,
        name=targetname_pair.value,
        origin=origin,
        targetname_pair=targetname_pair,
        origin_pair=origin_pair,
        blockers=blockers,
    )


def _transition_volume_from_entity(entity: SemanticEntity) -> TransitionVolume:
    names = (*_direct_values(entity, "targetname"), *_direct_values(entity, "landmark"))
    return TransitionVolume(
        entity_index=entity.index,
        hammer_id=entity.hammer_id,
        names=tuple(dict.fromkeys(names)),
    )


def _changelevel_from_entity(entity: SemanticEntity) -> ChangeLevelTrigger:
    destination_pair = _first_pair(entity, "map")
    landmark_pair = _first_pair(entity, "landmark")
    return ChangeLevelTrigger(
        entity_index=entity.index,
        hammer_id=entity.hammer_id,
        destination_raw=destination_pair.value if destination_pair is not None else None,
        destination_pair=destination_pair,
        landmark_name=landmark_pair.value if landmark_pair is not None else None,
        landmark_pair=landmark_pair,
        targetnames=_direct_values(entity, "targetname"),
    )


def _edge_from_changelevel(
    changelevel: ChangeLevelTrigger,
    landmarks: tuple[LandmarkDefinition, ...],
    transition_volumes: tuple[TransitionVolume, ...],
) -> TransitionEdge:
    blockers: list[TransitionBlocker] = []
    destination_raw = changelevel.destination_raw or ""
    landmark_name = changelevel.landmark_name or ""

    if changelevel.destination_raw is None or not changelevel.destination_raw:
        blockers.append(
            TransitionBlocker(
                code=TransitionBlockerCode.CHANGELEVEL_MISSING_MAP,
                message="trigger_changelevel is missing a direct map keyvalue.",
                entity_index=changelevel.entity_index,
            )
        )
    if changelevel.landmark_name is None or not changelevel.landmark_name:
        blockers.append(
            TransitionBlocker(
                code=TransitionBlockerCode.CHANGELEVEL_MISSING_LANDMARK,
                message="trigger_changelevel is missing a direct landmark keyvalue.",
                entity_index=changelevel.entity_index,
            )
        )

    matches = tuple(
        landmark for landmark in landmarks if landmark.name.casefold() == landmark_name.casefold()
    )
    landmark_origin: Vec3 | None = None
    if landmark_name:
        if not matches:
            blockers.append(
                TransitionBlocker(
                    code=TransitionBlockerCode.LANDMARK_NOT_FOUND,
                    message="No info_landmark matches the trigger_changelevel landmark name.",
                    entity_index=changelevel.entity_index,
                )
            )
        elif len(matches) > 1:
            blockers.append(
                TransitionBlocker(
                    code=TransitionBlockerCode.LANDMARK_AMBIGUOUS,
                    message=_AMBIGUOUS_LANDMARK_MESSAGE,
                    entity_index=changelevel.entity_index,
                )
            )
        else:
            matched_landmark = next(iter(matches))
            if matched_landmark.origin is None:
                blockers.extend(matched_landmark.blockers)
            else:
                landmark_origin = matched_landmark.origin

    transition_matches = tuple(
        volume
        for volume in transition_volumes
        if any(name.casefold() == landmark_name.casefold() for name in volume.names)
    )
    return TransitionEdge(
        changelevel_entity_index=changelevel.entity_index,
        changelevel_hammer_id=changelevel.hammer_id,
        destination_raw=destination_raw,
        destination_normalized=normalize_map_name(destination_raw) if destination_raw else "",
        landmark_name=landmark_name,
        landmark_matches=matches,
        landmark_origin=landmark_origin,
        transition_volume_matches=transition_matches,
        blockers=tuple(blockers),
    )
