from pathlib import Path

import pytest

from sourceweaver.errors import PatchConflictError, VmfSyntaxError
from sourceweaver.vmf import TextEdit, VmfDocument, apply_edits, parse

FIXTURE = Path(__file__).parent / "fixtures/minimal.vmf"


def test_fixture_roundtrips_byte_identically() -> None:
    document = VmfDocument.read(FIXTURE)
    assert document.render_bytes() == document.raw_bytes
    assert [block.key.value for block in document.syntax.blocks()] == [
        "versioninfo",
        "custom_editor_extension",
        "world",
        "cameras",
        "cordons",
    ]


def test_duplicate_keys_and_comments_are_retained() -> None:
    text = 'entity\n{\n\t"OnTrigger" "a,Kill,,0,-1" // one\n\t"OnTrigger" "b,Kill,,0,-1"\n}\n'
    parsed = parse(text)
    assert parsed.render() == text
    entity = parsed.blocks("ENTITY")[0]
    assert len(entity.entries) == 2


def test_crlf_and_cp1251_roundtrip() -> None:
    text = 'world\r\n{\r\n\t"message" "Привет"\r\n}\r\n'
    data = text.encode("cp1251")
    document = VmfDocument.from_bytes(data)
    assert document.encoding == "cp1251"
    assert document.newline == "\r\n"
    assert document.render_bytes() == data


def test_unterminated_string_is_rejected() -> None:
    with pytest.raises(VmfSyntaxError):
        parse('world { "message" "broken }')


def test_unbalanced_brace_is_rejected() -> None:
    with pytest.raises(VmfSyntaxError):
        parse("world\n{\n")


def test_edits_apply_back_to_front() -> None:
    assert apply_edits("abcdef", [TextEdit(1, 3, "XX"), TextEdit(5, 6, "Z")]) == "aXXdeZ"


def test_overlapping_edits_are_rejected() -> None:
    with pytest.raises(PatchConflictError):
        apply_edits("abcdef", [TextEdit(1, 4, "x"), TextEdit(3, 5, "y")])
