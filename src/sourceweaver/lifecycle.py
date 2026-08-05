"""Region lifecycle policy evidence for stitched Source maps.

The matrix is intentionally evidence-only. It classifies known classes into
preload/activate/deactivate policies and blocks unknown activation-affecting
classes instead of inventing runtime behavior.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum

from sourceweaver.semantics import EntityBlockKind, SemanticDocument


class LifecyclePolicyStatus(StrEnum):
    """Final status for lifecycle policy evidence."""

    CLEAR = "clear"
    BLOCKED = "blocked"


class LifecycleBlockerCode(StrEnum):
    """Deterministic blockers for lifecycle synthesis readiness."""

    UNKNOWN_LIFECYCLE_CLASS = "unknown_lifecycle_class"


class LifecycleControllerStatus(StrEnum):
    """Final status for lifecycle controller synthesis planning."""

    READY = "ready"
    BLOCKED = "blocked"


class LifecycleControllerBlockerCode(StrEnum):
    """Deterministic blockers for lifecycle controller planning."""

    EMPTY_REGION_NAME = "empty_region_name"
    POLICY_MATRIX_BLOCKED = "policy_matrix_blocked"


@dataclass(frozen=True, slots=True)
class LifecyclePolicy:
    """Lifecycle actions for one known class or class family."""

    family: str
    preload: str
    activate: str
    deactivate: str
    reset: str
    remove: str


@dataclass(frozen=True, slots=True)
class LifecyclePolicyEntry:
    """One entity matched to a lifecycle policy."""

    entity_index: int
    classname: str
    policy: LifecyclePolicy


@dataclass(frozen=True, slots=True)
class LifecycleBlocker:
    """A reason lifecycle synthesis cannot continue automatically."""

    code: LifecycleBlockerCode
    message: str
    entity_index: int
    classname: str


@dataclass(frozen=True, slots=True)
class LifecyclePolicyMatrix:
    """Evidence-only lifecycle policy matrix for one semantic VMF."""

    status: LifecyclePolicyStatus
    entries: tuple[LifecyclePolicyEntry, ...]
    blockers: tuple[LifecycleBlocker, ...]
    mutation_authorized: bool = False


@dataclass(frozen=True, slots=True)
class LifecycleControllerStep:
    """One deterministic controller action planned for a lifecycle phase."""

    order: int
    phase: str
    entity_index: int
    classname: str
    action: str


@dataclass(frozen=True, slots=True)
class LifecycleControllerBlocker:
    """A reason a lifecycle controller plan cannot be synthesized."""

    code: LifecycleControllerBlockerCode
    message: str


@dataclass(frozen=True, slots=True)
class LifecycleControllerPlan:
    """Read-only lifecycle controller plan for one imported region."""

    status: LifecycleControllerStatus
    region_name: str
    steps: tuple[LifecycleControllerStep, ...]
    blockers: tuple[LifecycleControllerBlocker, ...]
    mutation_authorized: bool = False


_TRANSITION_SCAFFOLD_CLASSES = frozenset(
    {
        "info_landmark",
        "trigger_changelevel",
        "trigger_transition",
    }
)
_POLICIES = {
    "logic_auto": LifecyclePolicy(
        family="startup_logic",
        preload="suppress automatic startup until region activation",
        activate="replay mapped startup outputs once per activation token",
        deactivate="preserve fired state unless reset policy is declared",
        reset="requires explicit reset policy before replay",
        remove="remove only when region is retired and outputs are no longer referenced",
    ),
    "ambient_generic": LifecyclePolicy(
        family="ambient_sound",
        preload="keep silent before region activation",
        activate="start or fade in on region activation",
        deactivate="stop or fade out when region becomes dormant",
        reset="restore authored sound state on reset",
        remove="remove only when region is retired",
    ),
    "env_fog_controller": LifecyclePolicy(
        family="environment_controller",
        preload="retain controller but do not select it globally",
        activate="select controller for active region",
        deactivate="hand off to next active region controller",
        reset="restore authored controller values",
        remove="remove only when no active region can reference it",
    ),
    "env_tonemap_controller": LifecyclePolicy(
        family="environment_controller",
        preload="retain controller but do not select it globally",
        activate="select controller for active region",
        deactivate="hand off to next active region controller",
        reset="restore authored controller values",
        remove="remove only when no active region can reference it",
    ),
    "trigger_once": LifecyclePolicy(
        family="trigger",
        preload="disable before region activation",
        activate="enable on region activation",
        deactivate="preserve fired state for backtracking",
        reset="restore unfired state only under explicit reset policy",
        remove="remove only when region is retired",
    ),
    "prop_door_rotating": LifecyclePolicy(
        family="door",
        preload="preserve authored physical state and disable external activation",
        activate="enable controls in dependency order",
        deactivate="keep physical state for backtracking",
        reset="restore authored state only under explicit reset policy",
        remove="do not remove while backtracking is possible",
    ),
}
_PREFIX_POLICIES = (
    (
        "npc_",
        LifecyclePolicy(
            family="npc",
            preload="start disabled or asleep when supported",
            activate="enable or spawn in authored dependency order",
            deactivate="sleep, disable, or persist according to state policy",
            reset="requires class-specific state reset policy",
            remove="remove only when disposable and region is retired",
        ),
    ),
    (
        "weapon_",
        LifecyclePolicy(
            family="pickup",
            preload="preserve item but keep outside-region activation suppressed",
            activate="enable pickup when region becomes active",
            deactivate="preserve picked-up state for backtracking",
            reset="restore pickup only under explicit reset policy",
            remove="remove only when consumed and region policy allows it",
        ),
    ),
    (
        "item_",
        LifecyclePolicy(
            family="pickup",
            preload="preserve item but keep outside-region activation suppressed",
            activate="enable pickup when region becomes active",
            deactivate="preserve picked-up state for backtracking",
            reset="restore pickup only under explicit reset policy",
            remove="remove only when consumed and region policy allows it",
        ),
    ),
)


def build_lifecycle_policy_matrix(document: SemanticDocument) -> LifecyclePolicyMatrix:
    """Classify known entity classes and block unknown lifecycle behavior."""
    entries: list[LifecyclePolicyEntry] = []
    blockers: list[LifecycleBlocker] = []
    for entity in document.entities:
        if entity.kind is not EntityBlockKind.ENTITY or entity.classname is None:
            continue
        classname = entity.classname.casefold()
        if classname in _TRANSITION_SCAFFOLD_CLASSES:
            continue
        policy = _policy_for_classname(classname)
        if policy is None:
            blockers.append(
                LifecycleBlocker(
                    code=LifecycleBlockerCode.UNKNOWN_LIFECYCLE_CLASS,
                    message="Entity classname has no lifecycle policy in the current registry.",
                    entity_index=entity.index,
                    classname=entity.classname,
                )
            )
            continue
        entries.append(
            LifecyclePolicyEntry(
                entity_index=entity.index,
                classname=entity.classname,
                policy=policy,
            )
        )
    if blockers:
        return LifecyclePolicyMatrix(
            status=LifecyclePolicyStatus.BLOCKED,
            entries=(),
            blockers=tuple(blockers),
        )
    return LifecyclePolicyMatrix(
        status=LifecyclePolicyStatus.CLEAR,
        entries=tuple(entries),
        blockers=(),
    )


def build_lifecycle_controller_plan(
    matrix: LifecyclePolicyMatrix,
    *,
    region_name: str,
) -> LifecycleControllerPlan:
    """Build deterministic read-only lifecycle controller steps for a region."""
    blockers: list[LifecycleControllerBlocker] = []
    if not region_name:
        blockers.append(
            LifecycleControllerBlocker(
                code=LifecycleControllerBlockerCode.EMPTY_REGION_NAME,
                message="Lifecycle controller region name must not be empty.",
            )
        )
    if matrix.status is not LifecyclePolicyStatus.CLEAR:
        blockers.append(
            LifecycleControllerBlocker(
                code=LifecycleControllerBlockerCode.POLICY_MATRIX_BLOCKED,
                message="Lifecycle policy matrix is blocked.",
            )
        )
    if blockers:
        return LifecycleControllerPlan(
            status=LifecycleControllerStatus.BLOCKED,
            region_name=region_name,
            steps=(),
            blockers=tuple(blockers),
        )
    return LifecycleControllerPlan(
        status=LifecycleControllerStatus.READY,
        region_name=region_name,
        steps=_controller_steps(matrix.entries),
        blockers=(),
    )


def _policy_for_classname(classname: str) -> LifecyclePolicy | None:
    if classname in _POLICIES:
        return _POLICIES[classname]
    for prefix, policy in _PREFIX_POLICIES:
        if classname.startswith(prefix):
            return policy
    return None


def _controller_steps(
    entries: tuple[LifecyclePolicyEntry, ...],
) -> tuple[LifecycleControllerStep, ...]:
    phases = (
        ("preload", "preload"),
        ("activate", "activate"),
        ("deactivate", "deactivate"),
        ("reset", "reset"),
        ("remove", "remove"),
    )
    steps: list[LifecycleControllerStep] = []
    order = 0
    for phase, field_name in phases:
        for entry in entries:
            steps.append(
                LifecycleControllerStep(
                    order=order,
                    phase=phase,
                    entity_index=entry.entity_index,
                    classname=entry.classname,
                    action=getattr(entry.policy, field_name),
                )
            )
            order += 1
    return tuple(steps)
