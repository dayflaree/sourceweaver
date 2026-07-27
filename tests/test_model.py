from pathlib import Path
from types import SimpleNamespace

import pytest

from sourceweaver.errors import ArtifactChangedError
from sourceweaver.model import ArtifactFingerprint


def test_fingerprint_from_bytes_and_path_match(tmp_path: Path) -> None:
    path = tmp_path / "artifact.bin"
    data = b"artifact contents"
    path.write_bytes(data)
    from_bytes = ArtifactFingerprint.from_bytes(data, path=path)
    from_path = ArtifactFingerprint.from_path(path)
    assert from_bytes == from_path


def test_fingerprint_rejects_mid_read_metadata_change(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "artifact.bin"
    path.write_bytes(b"artifact")
    before = SimpleNamespace(st_dev=1, st_ino=2, st_size=8, st_mtime_ns=3, st_ctime_ns=4)
    after = SimpleNamespace(st_dev=1, st_ino=2, st_size=8, st_mtime_ns=5, st_ctime_ns=4)
    states = iter((before, after))
    monkeypatch.setattr("sourceweaver.model.os.fstat", lambda _descriptor: next(states))
    with pytest.raises(ArtifactChangedError, match="changed while being fingerprinted"):
        ArtifactFingerprint.from_path(path)


def test_fingerprint_rejects_disappearing_path(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "artifact.bin"
    path.write_bytes(b"artifact")
    original_stat = Path.stat

    def fail_for_artifact(self: Path, *args: object, **kwargs: object) -> object:
        if self == path:
            raise FileNotFoundError(path)
        return original_stat(self, *args, **kwargs)

    monkeypatch.setattr(Path, "stat", fail_for_artifact)
    with pytest.raises(ArtifactChangedError, match="path disappeared"):
        ArtifactFingerprint.from_path(path)
