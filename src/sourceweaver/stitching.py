"""Read-only Source map transition graph extraction.

This module extracts transition facts from CST-backed semantic entities. It is
analysis-only and does not authorize alignment, stitching, VMF mutation, or
transition-volume deletion.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from enum import StrEnum

from sourceweaver.geometry import Vec3
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


def _classname_is(entity: SemanticEntity, classname: str) -> bool:
    return entity.classname is not None and entity.classname.casefold() == classname


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
