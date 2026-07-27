import os
from pathlib import Path

import pytest

from sourceweaver.errors import UnsafeOutputError
from sourceweaver.fileio import atomic_write_bytes, paths_refer_to_same_file


def test_paths_refer_to_same_literal_path(tmp_path: Path) -> None:
    source = tmp_path / "map.vmf"
    source.write_bytes(b"map")
    assert paths_refer_to_same_file(source, source)
    assert paths_refer_to_same_file(source, source.parent / "." / source.name)


def test_paths_refer_to_same_hard_link(tmp_path: Path) -> None:
    source = tmp_path / "map.vmf"
    link = tmp_path / "map-link.vmf"
    source.write_bytes(b"map")
    try:
        os.link(source, link)
    except OSError:
        pytest.skip("filesystem does not support hard links")
    assert paths_refer_to_same_file(source, link)


def test_different_paths_are_not_same_file(tmp_path: Path) -> None:
    first = tmp_path / "a.vmf"
    second = tmp_path / "b.vmf"
    first.write_bytes(b"a")
    second.write_bytes(b"a")
    assert not paths_refer_to_same_file(first, second)


def test_atomic_write_creates_new_file(tmp_path: Path) -> None:
    output = tmp_path / "nested" / "copy.vmf"
    assert atomic_write_bytes(output, b"content") == output
    assert output.read_bytes() == b"content"
    assert not list(output.parent.glob(f".{output.name}.*.tmp"))


def test_atomic_write_refuses_existing_file(tmp_path: Path) -> None:
    output = tmp_path / "copy.vmf"
    output.write_bytes(b"original")
    with pytest.raises(UnsafeOutputError, match="already exists"):
        atomic_write_bytes(output, b"replacement")
    assert output.read_bytes() == b"original"


def test_atomic_write_can_replace_when_explicit(tmp_path: Path) -> None:
    output = tmp_path / "copy.vmf"
    output.write_bytes(b"original")
    atomic_write_bytes(output, b"replacement", overwrite=True)
    assert output.read_bytes() == b"replacement"


def test_atomic_write_rejects_directory(tmp_path: Path) -> None:
    with pytest.raises(UnsafeOutputError, match="directory"):
        atomic_write_bytes(tmp_path, b"content", overwrite=True)


def test_atomic_write_falls_back_when_hard_links_are_unavailable(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    output = tmp_path / "copy.vmf"

    def fail_link(_source: object, _destination: object) -> None:
        raise OSError("hard links disabled")

    monkeypatch.setattr(os, "link", fail_link)
    atomic_write_bytes(output, b"fallback")
    assert output.read_bytes() == b"fallback"
