from pathlib import Path

import pytest

from sourceweaver.compiler import (
    CompilerRunBlockerCode,
    CompilerRunStatus,
    CompilerSet,
    build_compiler_run_preflight,
    discover_compilers,
    discover_gmod_root,
    executable_format,
    host_compatibility,
    parse_steam_library_paths,
)


def _make_gmod_root(path: Path) -> Path:
    (path / "bin" / "win64").mkdir(parents=True)
    (path / "garrysmod").mkdir()
    return path


def test_discover_explicit_gmod_root(tmp_path: Path) -> None:
    root = _make_gmod_root(tmp_path / "GarrysMod")
    assert discover_gmod_root(root, environ={}, home=tmp_path, steam_roots=[]) == root


def test_discover_inner_game_directory_normalizes_to_install_root(tmp_path: Path) -> None:
    root = _make_gmod_root(tmp_path / "GarrysMod")
    (root / "garrysmod" / "gameinfo.txt").write_text("GameInfo {}", encoding="utf-8")
    assert discover_gmod_root(root / "garrysmod", environ={}, home=tmp_path, steam_roots=[]) == root


def test_environment_root_is_discovered(tmp_path: Path) -> None:
    root = _make_gmod_root(tmp_path / "EnvironmentGMod")
    result = discover_gmod_root(
        environ={"SOURCEWEAVER_GMOD_ROOT": str(root)}, home=tmp_path, steam_roots=[]
    )
    assert result == root


def test_parse_modern_steam_libraryfolders_paths() -> None:
    text = r"""
"libraryfolders"
{
    "0" { "path" "C:\\Program Files (x86)\\Steam" }
    "1" { "path" "D:\\SteamLibrary" }
    "2" { "path" "D:\\SteamLibrary" }
}
"""
    assert [str(path) for path in parse_steam_library_paths(text)] == [
        r"C:\Program Files (x86)\Steam",
        r"D:\SteamLibrary",
    ]


def test_parse_legacy_steam_libraryfolders_paths() -> None:
    text = r""""LibraryFolders"
{
    "TimeNextStatsReport" "1234567890"
    "ContentStatsID" "12345678901234567890"
    "1" "D:\\SteamLibrary"
    "2" "E:\\Games\\Steam"
}
"""
    assert [str(path) for path in parse_steam_library_paths(text)] == [
        r"D:\SteamLibrary",
        r"E:\Games\Steam",
    ]


def test_arbitrary_existing_directory_is_not_a_gmod_root(tmp_path: Path) -> None:
    arbitrary = tmp_path / "not-gmod"
    arbitrary.mkdir()
    assert discover_gmod_root(arbitrary, environ={}, home=tmp_path, steam_roots=[]) is None


def test_discover_gmod_in_secondary_steam_library(tmp_path: Path) -> None:
    primary = tmp_path / "Steam"
    secondary = tmp_path / "Library"
    (primary / "steamapps").mkdir(parents=True)
    root = _make_gmod_root(secondary / "steamapps" / "common" / "GarrysMod")
    (primary / "steamapps" / "libraryfolders.vdf").write_text(
        f'"libraryfolders"\n{{\n"1" {{ "path" "{secondary.as_posix()}" }}\n}}\n',
        encoding="utf-8",
    )
    assert discover_gmod_root(environ={}, home=tmp_path, steam_roots=[primary]) == root


@pytest.mark.parametrize(
    "relative_steam_root",
    [
        Path(".local/share/Steam"),
        Path(".steam/steam"),
        Path(".steam/debian-installation"),
        Path(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
        Path("snap/steam/common/.local/share/Steam"),
    ],
)
def test_discover_gmod_in_common_linux_steam_roots(
    tmp_path: Path, relative_steam_root: Path
) -> None:
    root = _make_gmod_root(tmp_path / relative_steam_root / "steamapps" / "common" / "GarrysMod")
    assert discover_gmod_root(environ={}, home=tmp_path) == root


def test_discover_toolsplusplus_and_fingerprint(tmp_path: Path) -> None:
    root = _make_gmod_root(tmp_path / "GarrysMod")
    compiler_dir = root / "bin" / "win64"
    expected = {
        "vbspplusplus.exe": b"MZvbsp",
        "vvisplusplus.exe": b"MZvvis",
        "vradplusplus.exe": b"MZvrad",
        "bspzipplusplus.exe": b"MZbspzip",
    }
    for name, content in expected.items():
        (compiler_dir / name).write_bytes(content)

    compilers = discover_compilers(root)
    assert compilers.vbsp == compiler_dir / "vbspplusplus.exe"
    assert compilers.vvis == compiler_dir / "vvisplusplus.exe"
    assert compilers.vrad == compiler_dir / "vradplusplus.exe"
    assert compilers.bspzip == compiler_dir / "bspzipplusplus.exe"
    assert compilers.complete

    fingerprints = compilers.fingerprints()
    assert fingerprints["vbsp"] is not None
    assert fingerprints["vbsp"].size == 6
    assert len(fingerprints["vbsp"].sha256) == 64


def test_path_fallback_is_opt_in(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    root = _make_gmod_root(tmp_path / "GarrysMod")
    fake = tmp_path / "vbsp"
    fake.write_bytes(b"\x7fELF")
    monkeypatch.setattr("sourceweaver.compiler.shutil.which", lambda _name: str(fake))
    assert discover_compilers(root).vbsp is None
    assert discover_compilers(root, include_path=True).vbsp == fake


def test_missing_compilers_are_none(tmp_path: Path) -> None:
    root = _make_gmod_root(tmp_path / "GarrysMod")
    compilers = discover_compilers(root)
    assert compilers.vbsp is None
    assert compilers.vvis is None
    assert compilers.vrad is None
    assert compilers.bspzip is None
    assert not compilers.complete


@pytest.mark.parametrize(
    ("magic", "expected"),
    [
        (b"MZ\x00\x00", "windows-pe"),
        (b"\x7fELF", "linux-elf"),
        (b"\xcf\xfa\xed\xfe", "macos-mach-o"),
        (b"text", "unknown"),
    ],
)
def test_executable_format(tmp_path: Path, magic: bytes, expected: str) -> None:
    executable = tmp_path / "tool"
    executable.write_bytes(magic + b"payload")
    assert executable_format(executable) == expected


@pytest.mark.parametrize(
    ("kind", "host", "expected"),
    [
        ("windows-pe", "Windows", "native"),
        ("linux-elf", "Linux", "native"),
        ("windows-pe", "Linux", "compatibility-layer-required"),
        ("linux-elf", "Windows", "unsupported-format"),
        ("macos-mach-o", "Darwin", "native"),
        ("unknown", "Plan9", "unknown-host"),
    ],
)
def test_host_compatibility(kind: str, host: str, expected: str) -> None:
    assert host_compatibility(kind, host=host) == expected


def test_compiler_run_preflight_accepts_native_pipeline(tmp_path: Path) -> None:
    vbsp = tmp_path / "vbsp"
    vvis = tmp_path / "vvis"
    vrad = tmp_path / "vrad"
    for executable in (vbsp, vvis, vrad):
        executable.write_bytes(b"\x7fELFpayload")

    result = build_compiler_run_preflight(
        CompilerSet(vbsp=vbsp, vvis=vvis, vrad=vrad, bspzip=None), host="Linux"
    )

    assert result.status is CompilerRunStatus.READY
    assert result.runner_command is None
    assert result.blockers == ()
    assert [tool.role for tool in result.tools] == ["vbsp", "vvis", "vrad"]
    assert [tool.executable_format for tool in result.tools] == [
        "linux-elf",
        "linux-elf",
        "linux-elf",
    ]


def test_compiler_run_preflight_blocks_missing_required_tool(tmp_path: Path) -> None:
    vbsp = tmp_path / "vbsp"
    vbsp.write_bytes(b"\x7fELFpayload")

    result = build_compiler_run_preflight(
        CompilerSet(vbsp=vbsp, vvis=None, vrad=None, bspzip=None), host="Linux"
    )

    assert result.status is CompilerRunStatus.BLOCKED
    assert [(blocker.code, blocker.role) for blocker in result.blockers] == [
        (CompilerRunBlockerCode.MISSING_COMPILER, "vvis"),
        (CompilerRunBlockerCode.MISSING_COMPILER, "vrad"),
    ]


def test_compiler_run_preflight_blocks_windows_tools_without_runner(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    vbsp = tmp_path / "vbsp.exe"
    vvis = tmp_path / "vvis.exe"
    vrad = tmp_path / "vrad.exe"
    for executable in (vbsp, vvis, vrad):
        executable.write_bytes(b"MZpayload")
    monkeypatch.setattr("sourceweaver.compiler.shutil.which", lambda _name: None)

    result = build_compiler_run_preflight(
        CompilerSet(vbsp=vbsp, vvis=vvis, vrad=vrad, bspzip=None), host="Linux"
    )

    assert result.status is CompilerRunStatus.BLOCKED
    assert result.runner_command is None
    assert [(blocker.code, blocker.role) for blocker in result.blockers] == [
        (CompilerRunBlockerCode.COMPATIBILITY_LAYER_REQUIRED, "vbsp"),
        (CompilerRunBlockerCode.COMPATIBILITY_LAYER_REQUIRED, "vvis"),
        (CompilerRunBlockerCode.COMPATIBILITY_LAYER_REQUIRED, "vrad"),
    ]


def test_compiler_run_preflight_accepts_windows_tools_with_runner(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    vbsp = tmp_path / "vbsp.exe"
    vvis = tmp_path / "vvis.exe"
    vrad = tmp_path / "vrad.exe"
    for executable in (vbsp, vvis, vrad):
        executable.write_bytes(b"MZpayload")
    monkeypatch.setattr(
        "sourceweaver.compiler.shutil.which",
        lambda name: "/usr/bin/wine64" if name == "wine64" else None,
    )

    result = build_compiler_run_preflight(
        CompilerSet(vbsp=vbsp, vvis=vvis, vrad=vrad, bspzip=None), host="Linux"
    )

    assert result.status is CompilerRunStatus.READY
    assert result.runner_command == "wine64"
    assert result.blockers == ()
