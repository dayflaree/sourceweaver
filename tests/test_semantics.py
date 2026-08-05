from sourceweaver.semantics import build_semantic_document
from sourceweaver.vmf import VmfDocument


def test_cst_semantic_entities_preserve_duplicate_names_outputs_and_spans() -> None:
    document = VmfDocument.from_bytes(
        b"""world
{
    "id" "1"
    "classname" "worldspawn"
}
entity
{
    "id" "2"
    "classname" "logic_relay"
    "targetname" "relay_a"
    "targetname" "relay_alias"
}
entity
{
    "id" "3"
    "classname" "trigger_once"
    "targetname" "trigger"
    "parentname" "relay_a"
    "OnTrigger" "relay_a,Trigger,comma\\,kept,0.25,1"
    "OnTrigger" "missing_target,Kill,,0,-1"
}
""",
        path="synthetic.vmf",
    )

    semantic = build_semantic_document(document)

    assert [entity.classname for entity in semantic.entities] == [
        "worldspawn",
        "logic_relay",
        "trigger_once",
    ]
    assert [definition.name for definition in semantic.target_graph.definitions] == [
        "relay_a",
        "relay_alias",
        "trigger",
    ]
    assert [output.target for output in semantic.entities[2].outputs] == [
        "relay_a",
        "missing_target",
    ]
    assert semantic.entities[2].outputs[0].input_name == "Trigger"
    assert semantic.entities[2].outputs[0].parameter == "comma,kept"
    assert semantic.entities[2].outputs[0].delay == "0.25"
    assert semantic.entities[2].outputs[0].fire_count == "1"
    assert [reference.name for reference in semantic.target_graph.resolved_references] == [
        "relay_a",
        "relay_a",
    ]
    assert [reference.name for reference in semantic.target_graph.unresolved_references] == [
        "missing_target",
    ]

    spans = [pair.entry_span for entity in semantic.entities for pair in entity.keyvalues]
    assert len(spans) == len(set(spans))
    assert all(span.start < span.end for span in spans)
