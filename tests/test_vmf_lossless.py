import json
from pathlib import Path
from types import SimpleNamespace

import pytest
from hypothesis import given
from hypothesis import strategies as st

from sourceweaver.errors import (
    ArtifactChangedError,
    PatchConflictError,
    VmfLimitError,
    VmfSyntaxError,
)
from sourceweaver.vmf import TextEdit, VmfDocument, apply_edits, parse
from sourceweaver.vmf.lexer import TokenKind, lex

FIXTURE = Path(__file__).parent / "fixtures/minimal.vmf"
LOSSLESS_FIXTURE_ROOT = Path(__file__).parent / "fixtures/lossless"


def _lossless_fixture_cases() -> list[tuple[Path, dict[str, object]]]:
    manifest_path = LOSSLESS_FIXTURE_ROOT / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    cases: list[tuple[Path, dict[str, object]]] = []
    for fixture in manifest["fixtures"]:
        if not isinstance(fixture, dict):
            raise AssertionError(f"Invalid fixture entry in {manifest_path}: {fixture!r}")
        path = LOSSLESS_FIXTURE_ROOT / str(fixture["path"])
        cases.append((path, fixture))
    return cases


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


@pytest.mark.parametrize(("path", "metadata"), _lossless_fixture_cases())
def test_manifested_lossless_fixture_roundtrips_byte_identically(
    path: Path, metadata: dict[str, object]
) -> None:
    document = VmfDocument.read(path)
    assert document.render_bytes() == document.raw_bytes
    assert document.encoding == metadata["encoding"]
    assert document.newline_style == metadata["expected_newline_style"]
    assert [block.key.value for block in document.syntax.blocks()] == metadata[
        "expected_top_level_blocks"
    ]
    assert metadata["provenance"] == (
        "synthetic, authored specifically for SourceWeaver tests; contains no game content"
    )
    assert metadata["coverage"]


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
    assert document.newline_style == "CRLF"
    assert document.render_bytes() == data


def test_utf8_bom_roundtrip() -> None:
    data = b'\xef\xbb\xbfworld\n{\n"classname" "worldspawn"\n}\n'
    document = VmfDocument.from_bytes(data)
    assert document.encoding == "utf-8-sig"
    assert document.render_bytes() == data


@pytest.mark.parametrize(
    ("text", "preferred", "style"),
    [
        ('world\n{\n"classname" "worldspawn"\n}', "\n", "LF"),
        ('world\r\n{\r\n"classname" "worldspawn"\r\n}', "\r\n", "CRLF"),
        ('world\r{\r"classname" "worldspawn"\r}', "\r", "CR"),
        ('world\r\n{\n"classname" "worldspawn"\r\n}', "\r\n", "MIXED"),
        ('world { "classname" "worldspawn" }', "\n", "NONE"),
    ],
)
def test_newline_detection(text: str, preferred: str, style: str) -> None:
    document = VmfDocument.from_bytes(text.encode())
    assert document.newline == preferred
    assert document.newline_style == style
    assert document.render_bytes() == text.encode()


def test_string_value_decodes_known_escapes_and_preserves_unknown_ones() -> None:
    token = next(token for token in lex(r'"a\\b\"c\q"') if token.kind is TokenKind.STRING)
    assert token.value == 'a\\b"c\\q'


def test_unterminated_string_is_rejected() -> None:
    with pytest.raises(VmfSyntaxError, match="Unterminated quoted string"):
        parse('world { "message" "broken }')


def test_unbalanced_brace_is_rejected() -> None:
    with pytest.raises(VmfSyntaxError, match="Missing closing brace"):
        parse("world\n{\n")


def test_unexpected_closing_brace_is_rejected() -> None:
    with pytest.raises(VmfSyntaxError, match="Unexpected closing brace"):
        parse("}\n")


def test_cr_only_error_positions_are_counted_correctly() -> None:
    with pytest.raises(VmfSyntaxError) as error:
        parse('world\r{\r"key"\r}')
    assert error.value.line == 4
    assert error.value.column == 1


def test_source_character_limit() -> None:
    with pytest.raises(VmfLimitError, match="decoded characters"):
        parse('"a" "b"', max_chars=3)


def test_source_byte_limit() -> None:
    with pytest.raises(VmfLimitError, match="configured limit"):
        VmfDocument.from_bytes(b'"a" "b"', max_bytes=3)


def test_vmf_read_rejects_mid_read_metadata_change(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "map.vmf"
    path.write_text('world { "classname" "worldspawn" }', encoding="utf-8")
    before = SimpleNamespace(st_dev=1, st_ino=2, st_size=36, st_mtime_ns=3, st_ctime_ns=4)
    after = SimpleNamespace(st_dev=1, st_ino=2, st_size=36, st_mtime_ns=5, st_ctime_ns=4)
    states = iter((before, after))
    monkeypatch.setattr("sourceweaver.vmf.document.os.fstat", lambda _descriptor: next(states))
    with pytest.raises(ArtifactChangedError, match="changed while being read"):
        VmfDocument.read(path)


def test_token_limit() -> None:
    with pytest.raises(VmfLimitError, match="token count"):
        parse('"a" "b"', max_tokens=1)


def test_depth_limit() -> None:
    text = 'a { b { c { "key" "value" } } }'
    with pytest.raises(VmfLimitError, match="nesting depth"):
        parse(text, max_depth=2)


@pytest.mark.parametrize(
    "kwargs",
    [
        {"max_chars": -1},
        {"max_tokens": 0},
        {"max_depth": 0},
    ],
)
def test_invalid_parser_limits(kwargs: dict[str, int]) -> None:
    with pytest.raises(ValueError):
        parse('"a" "b"', **kwargs)


def test_edits_apply_back_to_front() -> None:
    assert apply_edits("abcdef", [TextEdit(1, 3, "XX"), TextEdit(5, 6, "Z")]) == "aXXdeZ"


def test_same_position_insertions_retain_caller_order() -> None:
    assert apply_edits("ab", [TextEdit(1, 1, "X"), TextEdit(1, 1, "Y")]) == "aXYb"


def test_overlapping_edits_are_rejected() -> None:
    with pytest.raises(PatchConflictError):
        apply_edits("abcdef", [TextEdit(1, 4, "x"), TextEdit(3, 5, "y")])


def test_out_of_bounds_edit_is_rejected() -> None:
    with pytest.raises(ValueError, match="outside source text"):
        apply_edits("abc", [TextEdit(2, 5, "x")])


_SAFE_TEXT = st.text(
    alphabet=st.characters(
        whitelist_categories=("L", "N"),
        whitelist_characters=" _-.,:;!@#$%^&*()[]/",
    ),
    max_size=80,
)


@given(st.lists(st.tuples(_SAFE_TEXT, _SAFE_TEXT), max_size=30))
def test_generated_quoted_pairs_roundtrip(pairs: list[tuple[str, str]]) -> None:
    lines = ["entity", "{"]
    for key, value in pairs:
        lines.append(f'\t"{key}" "{value}" // retained')
    lines.extend(["}", ""])
    text = "\n".join(lines)
    assert parse(text).render() == text
