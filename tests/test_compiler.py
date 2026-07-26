from pathlib import Path

from sourceweaver.compiler import discover_compilers, discover_gmod_root


def test_discover_explicit_gmod_root(tmp_path: Path) -> None:
    root = tmp_path / "GarrysMod"
    root.mkdir()
    assert discover_gmod_root(root) == root


def test_discover_toolsplusplus_and_fingerprint(tmp_path: Path) -> None:
    root = tmp_path / "GarrysMod"
    compiler_dir = root / "bin" / "win64"
    compiler_dir.mkdir(parents=True)
    expected = {
        "vbspplusplus.exe": b"vbsp",
        "vvisplusplus.exe": b"vvis",
        "vradplusplus.exe": b"vrad",
        "bspzipplusplus.exe": b"bspzip",
    }
    for name, content in expected.items():
        (compiler_dir / name).write_bytes(content)

    compilers = discover_compilers(root)
    assert compilers.vbsp == compiler_dir / "vbspplusplus.exe"
    assert compilers.vvis == compiler_dir / "vvisplusplus.exe"
    assert compilers.vrad == compiler_dir / "vradplusplus.exe"
    assert compilers.bspzip == compiler_dir / "bspzipplusplus.exe"

    fingerprints = compilers.fingerprints()
    assert fingerprints["vbsp"] is not None
    assert fingerprints["vbsp"].size == 4
    assert len(fingerprints["vbsp"].sha256) == 64


def test_missing_compilers_are_none(tmp_path: Path) -> None:
    root = tmp_path / "GarrysMod"
    root.mkdir()
    compilers = discover_compilers(root)
    assert compilers.vbsp is None
    assert compilers.vvis is None
    assert compilers.vrad is None
    assert compilers.bspzip is None
