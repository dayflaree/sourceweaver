from pathlib import Path

from sourceweaver.lifecycle import (
    LifecycleBlockerCode,
    LifecyclePolicyStatus,
    build_lifecycle_policy_matrix,
)
from sourceweaver.semantics import SemanticDocument, build_semantic_document
from sourceweaver.vmf import VmfDocument


def _semantic(text: str) -> SemanticDocument:
    return build_semantic_document(
        VmfDocument.from_bytes(text.encode("utf-8"), path=Path("synthetic.vmf"))
    )


def test_lifecycle_policy_matrix_classifies_known_region_entities() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_auto"
}
entity
{
    "id" "3"
    "classname" "ambient_generic"
}
entity
{
    "id" "4"
    "classname" "trigger_once"
}
entity
{
    "id" "5"
    "classname" "prop_door_rotating"
}
"""
    )

    matrix = build_lifecycle_policy_matrix(semantic)

    assert matrix.status is LifecyclePolicyStatus.CLEAR
    assert matrix.blockers == ()
    assert matrix.mutation_authorized is False
    assert [
        (entry.entity_index, entry.classname, entry.policy.family) for entry in matrix.entries
    ] == [
        (1, "logic_auto", "startup_logic"),
        (2, "ambient_generic", "ambient_sound"),
        (3, "trigger_once", "trigger"),
        (4, "prop_door_rotating", "door"),
    ]
    logic_auto = matrix.entries[0].policy
    assert logic_auto.preload == "suppress automatic startup until region activation"
    assert logic_auto.activate == "replay mapped startup outputs once per activation token"
    assert logic_auto.deactivate == "preserve fired state unless reset policy is declared"


def test_lifecycle_policy_matrix_blocks_unknown_activation_class() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "custom_region_controller"
}
"""
    )

    matrix = build_lifecycle_policy_matrix(semantic)

    assert matrix.status is LifecyclePolicyStatus.BLOCKED
    assert matrix.entries == ()
    assert [
        (blocker.code, blocker.entity_index, blocker.classname) for blocker in matrix.blockers
    ] == [(LifecycleBlockerCode.UNKNOWN_LIFECYCLE_CLASS, 1, "custom_region_controller")]


def test_lifecycle_policy_matrix_supports_prefix_families() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "npc_citizen"
}
entity
{
    "id" "3"
    "classname" "weapon_pistol"
}
entity
{
    "id" "4"
    "classname" "item_healthkit"
}
"""
    )

    matrix = build_lifecycle_policy_matrix(semantic)

    assert matrix.status is LifecyclePolicyStatus.CLEAR
    assert [(entry.classname, entry.policy.family) for entry in matrix.entries] == [
        ("npc_citizen", "npc"),
        ("weapon_pistol", "pickup"),
        ("item_healthkit", "pickup"),
    ]


def test_lifecycle_policy_matrix_ignores_transition_scaffolding_already_handled_elsewhere() -> None:
    semantic = _semantic(
        """world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "info_landmark"
}
entity
{
    "id" "3"
    "classname" "trigger_changelevel"
}
"""
    )

    matrix = build_lifecycle_policy_matrix(semantic)

    assert matrix.status is LifecyclePolicyStatus.CLEAR
    assert matrix.entries == ()
    assert matrix.blockers == ()
