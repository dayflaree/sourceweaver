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
    BrushSource,
    BrushSpatialRecord,
    GeometryTransformStatus,
    Vec3,
    classify_brush_relation,
    find_potential_brush_intersections,
    translate_convex_brush_for_analysis,
)
from sourceweaver.semantics import (
    EntityBlockKind,
    ReferenceKind,
    SemanticDocument,
    SemanticEntity,
    SemanticPair,
)

_AMBIGUOUS_LANDMARK_MESSAGE = (
    "More than one info_landmark matches the trigger_changelevel landmark name."
)
_WORLD_CONFLICT_KEYS = (
    "skyname",
    "detailmaterial",
    "detailvbsp",
    "maxpropscreenwidth",
)
_SINGLETON_CLASSNAMES = (
    "color_correction",
    "color_correction_volume",
    "env_fog_controller",
    "env_tonemap_controller",
    "logic_auto",
    "sky_camera",
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


class SeamConfidenceStatus(StrEnum):
    """Final status for bounded seam confidence review."""

    READY_FOR_REVIEW = "ready_for_review"
    BLOCKED = "blocked"


class SeamConfidenceBlockerCode(StrEnum):
    """Deterministic blockers for seam confidence evidence."""

    DELETION_EVIDENCE_BLOCKED = "deletion_evidence_blocked"
    EMPTY_SEAM_EVIDENCE = "empty_seam_evidence"
    UNSAFE_OVERLAP = "unsafe_overlap"
    SOURCE_REMOVAL_UNSUPPORTED = "source_removal_unsupported"


class ImportIdKind(StrEnum):
    """VMF object ID classes planned for imported candidate data."""

    ENTITY = "entity"
    SOLID = "solid"
    SIDE = "side"


class ImportIdAllocationStatus(StrEnum):
    """Final status for an imported ID allocation plan."""

    VALID = "valid"
    BLOCKED = "blocked"


class ImportIdAllocationBlockerCode(StrEnum):
    """Deterministic blockers for imported ID allocation."""

    MISSING_CANDIDATE_ID = "missing_candidate_id"
    NON_NUMERIC_ID = "non_numeric_id"
    DUPLICATE_CANDIDATE_ID = "duplicate_candidate_id"


class TargetNameNamespaceStatus(StrEnum):
    """Final status for a targetname namespace plan."""

    VALID = "valid"
    BLOCKED = "blocked"


class TargetNameNamespaceEditKind(StrEnum):
    """Kind of source-backed targetname namespace edit."""

    DEFINITION = "definition"
    REFERENCE = "reference"


class TargetNameNamespaceBlockerCode(StrEnum):
    """Deterministic blockers for targetname namespace planning."""

    EMPTY_PREFIX = "empty_prefix"
    NAMESPACED_NAME_COLLISION = "namespaced_name_collision"
    UNRESOLVED_REFERENCE = "unresolved_reference"
    AMBIGUOUS_REFERENCE = "ambiguous_reference"
    SPECIAL_REFERENCE_UNSUPPORTED = "special_reference_unsupported"
    WILDCARD_REFERENCE_UNSUPPORTED = "wildcard_reference_unsupported"


class SingletonConflictStatus(StrEnum):
    """Final status for world/singleton conflict evidence."""

    CLEAR = "clear"
    BLOCKED = "blocked"


class SingletonConflictCode(StrEnum):
    """Deterministic conflict classes for world/singleton systems."""

    WORLD_KEY_CONFLICT = "world_key_conflict"
    SINGLETON_CLASS_CONFLICT = "singleton_class_conflict"
    CANDIDATE_DUPLICATE_SINGLETON = "candidate_duplicate_singleton"


class StitchPreflightStatus(StrEnum):
    """Final status for aggregate stitch preflight."""

    READY_FOR_PLAN = "ready_for_plan"
    BLOCKED = "blocked"


class StitchPreflightBlockerCode(StrEnum):
    """Deterministic blockers for aggregate stitch preflight."""

    ALIGNMENT_BLOCKED = "alignment_blocked"
    SEAM_CONFIDENCE_BLOCKED = "seam_confidence_blocked"
    ID_ALLOCATION_BLOCKED = "id_allocation_blocked"
    NAMESPACE_BLOCKED = "namespace_blocked"
    SINGLETON_CONFLICT = "singleton_conflict"
    CAPACITY_EXCEEDED = "capacity_exceeded"


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


@dataclass(frozen=True, slots=True)
class SeamConfidenceBlocker:
    """A reason seam confidence cannot advance to review-ready state."""

    code: SeamConfidenceBlockerCode
    message: str
    item_index: int | None = None


@dataclass(frozen=True, slots=True)
class SeamConfidenceReport:
    """Bounded seam confidence summary with no mutation authority."""

    status: SeamConfidenceStatus
    pair_count: int
    candidate_removal_count: int
    source_removal_count: int
    touching_pair_count: int
    unsafe_overlap_count: int
    blockers: tuple[SeamConfidenceBlocker, ...]
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class ImportIdAllocation:
    """One planned candidate ID replacement."""

    kind: ImportIdKind
    original_id: str
    allocated_id: str


@dataclass(frozen=True, slots=True)
class ImportIdAllocationBlocker:
    """A reason imported ID allocation cannot proceed safely."""

    code: ImportIdAllocationBlockerCode
    message: str
    kind: ImportIdKind
    raw_id: str | None = None


@dataclass(frozen=True, slots=True)
class ImportIdAllocationPlan:
    """Plan-only fresh ID allocation for imported candidate objects."""

    status: ImportIdAllocationStatus
    allocations: tuple[ImportIdAllocation, ...]
    blockers: tuple[ImportIdAllocationBlocker, ...]
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class TargetNameNamespaceEdit:
    """One source-backed targetname namespace edit planned for later materialization."""

    kind: TargetNameNamespaceEditKind
    entity_index: int
    original_value: str
    namespaced_value: str
    reference_kind: ReferenceKind | None = None


@dataclass(frozen=True, slots=True)
class TargetNameNamespaceBlocker:
    """A reason a targetname namespace plan cannot be trusted."""

    code: TargetNameNamespaceBlockerCode
    message: str
    entity_index: int | None = None
    name: str | None = None


@dataclass(frozen=True, slots=True)
class TargetNameNamespacePlan:
    """CST-backed targetname namespace plan with no mutation authority."""

    status: TargetNameNamespaceStatus
    prefix: str
    edits: tuple[TargetNameNamespaceEdit, ...]
    blockers: tuple[TargetNameNamespaceBlocker, ...]
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class SingletonConflict:
    """One world/singleton conflict requiring human or later policy review."""

    code: SingletonConflictCode
    message: str
    key: str | None = None
    classname: str | None = None
    source_value: str | None = None
    candidate_value: str | None = None
    source_entity_indexes: tuple[int, ...] = ()
    candidate_entity_indexes: tuple[int, ...] = ()


@dataclass(frozen=True, slots=True)
class SingletonConflictReport:
    """Read-only world/singleton conflict evidence with no mutation authority."""

    status: SingletonConflictStatus
    conflicts: tuple[SingletonConflict, ...]
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class StitchPreflightBlocker:
    """A reason a stitch cannot advance to materialization planning."""

    code: StitchPreflightBlockerCode
    message: str
    detail: str | None = None


@dataclass(frozen=True, slots=True)
class StitchPreflightReport:
    """Aggregate read-only readiness report for stitch planning."""

    status: StitchPreflightStatus
    imported_entity_count: int
    imported_solid_count: int
    imported_side_count: int
    blockers: tuple[StitchPreflightBlocker, ...]
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class _ImportIdRecord:
    kind: ImportIdKind
    raw_id: str | None


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


def build_import_id_allocation_plan(
    source_document: SemanticDocument,
    candidate_document: SemanticDocument,
    source_brushes: Iterable[BrushSource],
    candidate_brushes: Iterable[BrushSource],
) -> ImportIdAllocationPlan:
    """Plan fresh IDs for imported candidate entities, solids, and sides.

    Candidate worldspawn IDs are not allocated here because world/singleton
    reconciliation is handled by a later stitch planning gate. Candidate world
    solids and sides are allocated because they may be imported into the source
    world geometry container.
    """
    source_records = tuple(_id_records_from_document(source_document, source_brushes))
    candidate_records = tuple(_import_candidate_id_records(candidate_document, candidate_brushes))
    blockers = (*_source_id_blockers(source_records), *_candidate_id_blockers(candidate_records))
    parsed_ids = tuple(_parse_id(record.raw_id) for record in (*source_records, *candidate_records))
    if blockers:
        return ImportIdAllocationPlan(
            status=ImportIdAllocationStatus.BLOCKED,
            allocations=(),
            blockers=blockers,
        )

    max_id = max((parsed_id for parsed_id in parsed_ids if parsed_id is not None), default=0)
    allocations: list[ImportIdAllocation] = []
    next_id = max_id + 1
    for record in candidate_records:
        if record.raw_id is None:
            return ImportIdAllocationPlan(
                status=ImportIdAllocationStatus.BLOCKED,
                allocations=(),
                blockers=(
                    ImportIdAllocationBlocker(
                        code=ImportIdAllocationBlockerCode.MISSING_CANDIDATE_ID,
                        message="Candidate import record is missing an ID.",
                        kind=record.kind,
                    ),
                ),
            )
        allocations.append(
            ImportIdAllocation(
                kind=record.kind,
                original_id=record.raw_id,
                allocated_id=str(next_id),
            )
        )
        next_id += 1
    return ImportIdAllocationPlan(
        status=ImportIdAllocationStatus.VALID,
        allocations=tuple(allocations),
        blockers=(),
    )


def build_targetname_namespace_plan(
    source_document: SemanticDocument,
    candidate_document: SemanticDocument,
    *,
    prefix: str,
) -> TargetNameNamespacePlan:
    """Plan candidate targetname namespacing for currently typed references.

    This uses the CST-backed semantic graph: direct `targetname` definitions,
    direct `parentname` references, and output targets. Full FGD-backed keyvalue
    rewriting remains a later gate.
    """
    blockers = list(_namespace_prefix_blockers(prefix))
    source_names = {
        definition.name.casefold() for definition in source_document.target_graph.definitions
    }
    blockers.extend(_namespace_collision_blockers(candidate_document, source_names, prefix))
    blockers.extend(_namespace_reference_blockers(candidate_document))
    if blockers:
        return TargetNameNamespacePlan(
            status=TargetNameNamespaceStatus.BLOCKED,
            prefix=prefix,
            edits=(),
            blockers=tuple(blockers),
        )
    return TargetNameNamespacePlan(
        status=TargetNameNamespaceStatus.VALID,
        prefix=prefix,
        edits=(
            *_namespace_definition_edits(candidate_document, prefix),
            *_namespace_reference_edits(candidate_document, prefix),
        ),
        blockers=(),
    )


def build_singleton_conflict_report(
    source_document: SemanticDocument,
    candidate_document: SemanticDocument,
) -> SingletonConflictReport:
    """Report baseline worldspawn and singleton-controller conflicts.

    The report is evidence for later stitch planning. It does not select a
    reconciliation policy and carries no mutation authority.
    """
    conflicts = (
        *_world_key_conflicts(source_document, candidate_document),
        *_singleton_class_conflicts(source_document, candidate_document),
        *_candidate_duplicate_singleton_conflicts(candidate_document),
    )
    return SingletonConflictReport(
        status=SingletonConflictStatus.BLOCKED if conflicts else SingletonConflictStatus.CLEAR,
        conflicts=conflicts,
    )


def build_stitch_preflight_report(
    alignment: TranslationAlignmentHypothesis,
    seam_confidence: SeamConfidenceReport,
    id_allocation: ImportIdAllocationPlan,
    namespace_plan: TargetNameNamespacePlan,
    singleton_conflicts: SingletonConflictReport,
    candidate_document: SemanticDocument,
    candidate_brushes: Iterable[BrushSource],
    *,
    max_imported_entities: int = 8192,
    max_imported_solids: int = 65535,
    max_imported_sides: int = 262144,
) -> StitchPreflightReport:
    """Aggregate evidence gates and capacity limits before stitch planning.

    This is a read-only preflight. It reports whether evidence is complete enough
    to build a stitch plan manifest, but it does not materialize VMF changes.
    """
    brushes = tuple(candidate_brushes)
    imported_entity_count = _imported_entity_count(candidate_document)
    imported_solid_count = len(brushes)
    imported_side_count = sum(len(brush.sides) for brush in brushes)
    blockers = (
        *_evidence_gate_blockers(
            alignment,
            seam_confidence,
            id_allocation,
            namespace_plan,
            singleton_conflicts,
        ),
        *_capacity_blockers(
            imported_entity_count=imported_entity_count,
            imported_solid_count=imported_solid_count,
            imported_side_count=imported_side_count,
            max_imported_entities=max_imported_entities,
            max_imported_solids=max_imported_solids,
            max_imported_sides=max_imported_sides,
        ),
    )
    return StitchPreflightReport(
        status=StitchPreflightStatus.BLOCKED if blockers else StitchPreflightStatus.READY_FOR_PLAN,
        imported_entity_count=imported_entity_count,
        imported_solid_count=imported_solid_count,
        imported_side_count=imported_side_count,
        blockers=blockers,
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


def build_seam_confidence_report(
    deletion_evidence: SeamDeletionEvidence,
) -> SeamConfidenceReport:
    """Summarize seam evidence and block unsupported overlap classes."""
    pair_count = len(deletion_evidence.items)
    candidate_removal_count = sum(1 for item in deletion_evidence.items if item.remove_candidate)
    source_removal_count = sum(1 for item in deletion_evidence.items if item.remove_source)
    touching_pair_count = sum(
        1
        for item in deletion_evidence.items
        if item.deletion_class is SeamDeletionClass.PRESERVE_TOUCHING_SEAM
    )
    unsafe_overlap_count = sum(
        1
        for item in deletion_evidence.items
        if item.deletion_class is SeamDeletionClass.PRESERVE_UNSAFE_OVERLAP
    )
    blockers: list[SeamConfidenceBlocker] = []
    if deletion_evidence.status is not SeamDeletionEvidenceStatus.VALID:
        blockers.append(
            SeamConfidenceBlocker(
                code=SeamConfidenceBlockerCode.DELETION_EVIDENCE_BLOCKED,
                message="Seam deletion evidence is blocked.",
            )
        )
    if deletion_evidence.status is SeamDeletionEvidenceStatus.VALID and pair_count == 0:
        blockers.append(
            SeamConfidenceBlocker(
                code=SeamConfidenceBlockerCode.EMPTY_SEAM_EVIDENCE,
                message="No seam brush pairs were found for confidence review.",
            )
        )
    blockers.extend(_confidence_item_blockers(deletion_evidence.items))
    return SeamConfidenceReport(
        status=(
            SeamConfidenceStatus.BLOCKED if blockers else SeamConfidenceStatus.READY_FOR_REVIEW
        ),
        pair_count=pair_count,
        candidate_removal_count=candidate_removal_count,
        source_removal_count=source_removal_count,
        touching_pair_count=touching_pair_count,
        unsafe_overlap_count=unsafe_overlap_count,
        blockers=tuple(blockers),
    )


def _classname_is(entity: SemanticEntity, classname: str) -> bool:
    return entity.classname is not None and entity.classname.casefold() == classname


def _id_records_from_document(
    document: SemanticDocument, brush_sources: Iterable[BrushSource]
) -> tuple[_ImportIdRecord, ...]:
    records: list[_ImportIdRecord] = [
        _ImportIdRecord(ImportIdKind.ENTITY, entity.hammer_id)
        for entity in document.entities
        if entity.hammer_id is not None
    ]
    records.extend(_brush_id_records(brush_sources))
    return tuple(records)


def _import_candidate_id_records(
    document: SemanticDocument, brush_sources: Iterable[BrushSource]
) -> tuple[_ImportIdRecord, ...]:
    records = list(_brush_id_records(brush_sources))
    records.extend(
        _ImportIdRecord(ImportIdKind.ENTITY, entity.hammer_id)
        for entity in document.entities
        if entity.kind is EntityBlockKind.ENTITY
    )
    return tuple(records)


def _brush_id_records(brush_sources: Iterable[BrushSource]) -> tuple[_ImportIdRecord, ...]:
    records: list[_ImportIdRecord] = []
    for brush in brush_sources:
        records.append(_ImportIdRecord(ImportIdKind.SOLID, brush.solid_id))
        records.extend(_ImportIdRecord(ImportIdKind.SIDE, side.side_id) for side in brush.sides)
    return tuple(records)


def _source_id_blockers(
    records: tuple[_ImportIdRecord, ...],
) -> tuple[ImportIdAllocationBlocker, ...]:
    return tuple(
        ImportIdAllocationBlocker(
            code=ImportIdAllocationBlockerCode.NON_NUMERIC_ID,
            message="Source VMF object ID is not a positive decimal integer.",
            kind=record.kind,
            raw_id=record.raw_id,
        )
        for record in records
        if record.raw_id is not None and _parse_id(record.raw_id) is None
    )


def _candidate_id_blockers(
    records: tuple[_ImportIdRecord, ...],
) -> tuple[ImportIdAllocationBlocker, ...]:
    blockers: list[ImportIdAllocationBlocker] = []
    seen: set[str] = set()
    for record in records:
        if record.raw_id is None:
            blockers.append(
                ImportIdAllocationBlocker(
                    code=ImportIdAllocationBlockerCode.MISSING_CANDIDATE_ID,
                    message="Candidate import record is missing an ID.",
                    kind=record.kind,
                )
            )
            continue
        if _parse_id(record.raw_id) is None:
            blockers.append(
                ImportIdAllocationBlocker(
                    code=ImportIdAllocationBlockerCode.NON_NUMERIC_ID,
                    message="Candidate VMF object ID is not a positive decimal integer.",
                    kind=record.kind,
                    raw_id=record.raw_id,
                )
            )
            continue
        if record.raw_id in seen:
            blockers.append(
                ImportIdAllocationBlocker(
                    code=ImportIdAllocationBlockerCode.DUPLICATE_CANDIDATE_ID,
                    message="Candidate import ID appears more than once.",
                    kind=record.kind,
                    raw_id=record.raw_id,
                )
            )
        seen.add(record.raw_id)
    return tuple(blockers)


def _parse_id(raw_id: str | None) -> int | None:
    if raw_id is None or not raw_id.isdecimal():
        return None
    parsed = int(raw_id)
    if parsed <= 0:
        return None
    return parsed


def _world_key_conflicts(
    source_document: SemanticDocument,
    candidate_document: SemanticDocument,
) -> tuple[SingletonConflict, ...]:
    source_world = _world_entity(source_document)
    candidate_world = _world_entity(candidate_document)
    if source_world is None or candidate_world is None:
        return ()
    conflicts: list[SingletonConflict] = []
    for key in _WORLD_CONFLICT_KEYS:
        source_value = _first_entity_value(source_world, key)
        candidate_value = _first_entity_value(candidate_world, key)
        if source_value is None or candidate_value is None or source_value == candidate_value:
            continue
        conflicts.append(
            SingletonConflict(
                code=SingletonConflictCode.WORLD_KEY_CONFLICT,
                message="Source and candidate worldspawn values differ.",
                key=key,
                source_value=source_value,
                candidate_value=candidate_value,
                source_entity_indexes=(source_world.index,),
                candidate_entity_indexes=(candidate_world.index,),
            )
        )
    return tuple(conflicts)


def _singleton_class_conflicts(
    source_document: SemanticDocument,
    candidate_document: SemanticDocument,
) -> tuple[SingletonConflict, ...]:
    source_by_class = _singleton_entities_by_class(source_document)
    candidate_by_class = _singleton_entities_by_class(candidate_document)
    conflicts: list[SingletonConflict] = []
    for classname in _SINGLETON_CLASSNAMES:
        source_indexes = source_by_class.get(classname, ())
        candidate_indexes = candidate_by_class.get(classname, ())
        if not source_indexes or not candidate_indexes:
            continue
        conflicts.append(
            SingletonConflict(
                code=SingletonConflictCode.SINGLETON_CLASS_CONFLICT,
                message="Source and candidate both define a known singleton class.",
                classname=classname,
                source_entity_indexes=source_indexes,
                candidate_entity_indexes=candidate_indexes,
            )
        )
    return tuple(conflicts)


def _candidate_duplicate_singleton_conflicts(
    candidate_document: SemanticDocument,
) -> tuple[SingletonConflict, ...]:
    candidate_by_class = _singleton_entities_by_class(candidate_document)
    conflicts: list[SingletonConflict] = []
    for classname in _SINGLETON_CLASSNAMES:
        candidate_indexes = candidate_by_class.get(classname, ())
        if len(candidate_indexes) <= 1:
            continue
        conflicts.append(
            SingletonConflict(
                code=SingletonConflictCode.CANDIDATE_DUPLICATE_SINGLETON,
                message="Candidate map defines more than one known singleton class instance.",
                classname=classname,
                candidate_entity_indexes=candidate_indexes,
            )
        )
    return tuple(conflicts)


def _world_entity(document: SemanticDocument) -> SemanticEntity | None:
    for entity in document.entities:
        if entity.kind is EntityBlockKind.WORLD:
            return entity
    return None


def _first_entity_value(entity: SemanticEntity, key: str) -> str | None:
    pair = _first_pair(entity, key)
    return pair.value if pair is not None else None


def _singleton_entities_by_class(
    document: SemanticDocument,
) -> dict[str, tuple[int, ...]]:
    indexes_by_class: dict[str, list[int]] = {}
    singleton_names = set(_SINGLETON_CLASSNAMES)
    for entity in document.entities:
        if entity.kind is not EntityBlockKind.ENTITY or entity.classname is None:
            continue
        classname = entity.classname.casefold()
        if classname not in singleton_names:
            continue
        indexes_by_class.setdefault(classname, []).append(entity.index)
    return {classname: tuple(indexes) for classname, indexes in sorted(indexes_by_class.items())}


def _imported_entity_count(candidate_document: SemanticDocument) -> int:
    return sum(1 for entity in candidate_document.entities if entity.kind is EntityBlockKind.ENTITY)


def _evidence_gate_blockers(
    alignment: TranslationAlignmentHypothesis,
    seam_confidence: SeamConfidenceReport,
    id_allocation: ImportIdAllocationPlan,
    namespace_plan: TargetNameNamespacePlan,
    singleton_conflicts: SingletonConflictReport,
) -> tuple[StitchPreflightBlocker, ...]:
    blockers: list[StitchPreflightBlocker] = []
    if alignment.status is not AlignmentStatus.VALID:
        blockers.append(
            StitchPreflightBlocker(
                code=StitchPreflightBlockerCode.ALIGNMENT_BLOCKED,
                message="Translation alignment hypothesis is blocked.",
            )
        )
    if seam_confidence.status is not SeamConfidenceStatus.READY_FOR_REVIEW:
        blockers.append(
            StitchPreflightBlocker(
                code=StitchPreflightBlockerCode.SEAM_CONFIDENCE_BLOCKED,
                message="Seam confidence evidence is blocked.",
            )
        )
    if id_allocation.status is not ImportIdAllocationStatus.VALID:
        blockers.append(
            StitchPreflightBlocker(
                code=StitchPreflightBlockerCode.ID_ALLOCATION_BLOCKED,
                message="Imported ID allocation plan is blocked.",
            )
        )
    if namespace_plan.status is not TargetNameNamespaceStatus.VALID:
        blockers.append(
            StitchPreflightBlocker(
                code=StitchPreflightBlockerCode.NAMESPACE_BLOCKED,
                message="Targetname namespace plan is blocked.",
            )
        )
    if singleton_conflicts.status is not SingletonConflictStatus.CLEAR:
        blockers.append(
            StitchPreflightBlocker(
                code=StitchPreflightBlockerCode.SINGLETON_CONFLICT,
                message="World/singleton conflict evidence is blocked.",
            )
        )
    return tuple(blockers)


def _capacity_blockers(
    *,
    imported_entity_count: int,
    imported_solid_count: int,
    imported_side_count: int,
    max_imported_entities: int,
    max_imported_solids: int,
    max_imported_sides: int,
) -> tuple[StitchPreflightBlocker, ...]:
    blockers: list[StitchPreflightBlocker] = []
    if imported_entity_count > max_imported_entities:
        blockers.append(
            StitchPreflightBlocker(
                code=StitchPreflightBlockerCode.CAPACITY_EXCEEDED,
                message="Candidate entity import count exceeds the configured limit.",
                detail=f"entities={imported_entity_count} limit={max_imported_entities}",
            )
        )
    if imported_solid_count > max_imported_solids:
        blockers.append(
            StitchPreflightBlocker(
                code=StitchPreflightBlockerCode.CAPACITY_EXCEEDED,
                message="Candidate solid import count exceeds the configured limit.",
                detail=f"solids={imported_solid_count} limit={max_imported_solids}",
            )
        )
    if imported_side_count > max_imported_sides:
        blockers.append(
            StitchPreflightBlocker(
                code=StitchPreflightBlockerCode.CAPACITY_EXCEEDED,
                message="Candidate side import count exceeds the configured limit.",
                detail=f"sides={imported_side_count} limit={max_imported_sides}",
            )
        )
    return tuple(blockers)


def _namespace_prefix_blockers(prefix: str) -> tuple[TargetNameNamespaceBlocker, ...]:
    if prefix:
        return ()
    return (
        TargetNameNamespaceBlocker(
            code=TargetNameNamespaceBlockerCode.EMPTY_PREFIX,
            message="Candidate targetname namespace prefix must not be empty.",
        ),
    )


def _namespace_collision_blockers(
    candidate_document: SemanticDocument,
    source_names: set[str],
    prefix: str,
) -> tuple[TargetNameNamespaceBlocker, ...]:
    return tuple(
        TargetNameNamespaceBlocker(
            code=TargetNameNamespaceBlockerCode.NAMESPACED_NAME_COLLISION,
            message="Namespaced candidate targetname collides with a source targetname.",
            entity_index=definition.entity_index,
            name=f"{prefix}{definition.name}",
        )
        for definition in candidate_document.target_graph.definitions
        if f"{prefix}{definition.name}".casefold() in source_names
    )


def _namespace_reference_blockers(
    candidate_document: SemanticDocument,
) -> tuple[TargetNameNamespaceBlocker, ...]:
    blockers: list[TargetNameNamespaceBlocker] = []
    blockers.extend(
        TargetNameNamespaceBlocker(
            code=TargetNameNamespaceBlockerCode.UNRESOLVED_REFERENCE,
            message="Candidate targetname reference does not resolve uniquely.",
            entity_index=reference.entity_index,
            name=reference.name,
        )
        for reference in candidate_document.target_graph.unresolved_references
    )
    blockers.extend(
        TargetNameNamespaceBlocker(
            code=TargetNameNamespaceBlockerCode.AMBIGUOUS_REFERENCE,
            message="Candidate targetname reference is ambiguous.",
            entity_index=reference.entity_index,
            name=reference.name,
        )
        for reference in candidate_document.target_graph.ambiguous_references
    )
    for reference in candidate_document.target_graph.references:
        if reference.kind is ReferenceKind.SPECIAL:
            blockers.append(
                TargetNameNamespaceBlocker(
                    code=TargetNameNamespaceBlockerCode.SPECIAL_REFERENCE_UNSUPPORTED,
                    message="Special targetname reference requires a later typed rewrite gate.",
                    entity_index=reference.entity_index,
                    name=reference.name,
                )
            )
        if reference.kind is ReferenceKind.WILDCARD:
            blockers.append(
                TargetNameNamespaceBlocker(
                    code=TargetNameNamespaceBlockerCode.WILDCARD_REFERENCE_UNSUPPORTED,
                    message="Wildcard targetname reference requires a later typed rewrite gate.",
                    entity_index=reference.entity_index,
                    name=reference.name,
                )
            )
    return tuple(blockers)


def _namespace_definition_edits(
    candidate_document: SemanticDocument, prefix: str
) -> tuple[TargetNameNamespaceEdit, ...]:
    return tuple(
        TargetNameNamespaceEdit(
            kind=TargetNameNamespaceEditKind.DEFINITION,
            entity_index=definition.entity_index,
            original_value=definition.name,
            namespaced_value=f"{prefix}{definition.name}",
        )
        for definition in candidate_document.target_graph.definitions
    )


def _namespace_reference_edits(
    candidate_document: SemanticDocument, prefix: str
) -> tuple[TargetNameNamespaceEdit, ...]:
    return tuple(
        TargetNameNamespaceEdit(
            kind=TargetNameNamespaceEditKind.REFERENCE,
            entity_index=reference.entity_index,
            original_value=reference.name,
            namespaced_value=f"{prefix}{reference.name}",
            reference_kind=reference.kind,
        )
        for reference in candidate_document.target_graph.resolved_references
    )


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


def _confidence_item_blockers(
    items: tuple[SeamDeletionItemEvidence, ...],
) -> tuple[SeamConfidenceBlocker, ...]:
    blockers: list[SeamConfidenceBlocker] = []
    for index, item in enumerate(items):
        if item.deletion_class is SeamDeletionClass.PRESERVE_UNSAFE_OVERLAP:
            blockers.append(
                SeamConfidenceBlocker(
                    code=SeamConfidenceBlockerCode.UNSAFE_OVERLAP,
                    message="Unsupported overlapping seam brush pair requires review.",
                    item_index=index,
                )
            )
        if item.remove_source:
            blockers.append(
                SeamConfidenceBlocker(
                    code=SeamConfidenceBlockerCode.SOURCE_REMOVAL_UNSUPPORTED,
                    message="Source-map brush removal is unsupported in this stitcher slice.",
                    item_index=index,
                )
            )
    return tuple(blockers)


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
