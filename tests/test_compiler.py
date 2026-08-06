from pathlib import Path

import pytest

from sourceweaver.compiler import (
    CompilerArtifactBlockerCode,
    CompilerArtifactStatus,
    CompilerInvocationBlockerCode,
    CompilerInvocationStatus,
    CompilerLogMessageCode,
    CompilerLogStatus,
    CompilerRunBlockerCode,
    CompilerRunPreflight,
    CompilerRunStatus,
    CompilerRunTool,
    CompilerSet,
    build_compile_invocation_plan,
    build_compiler_run_preflight,
    discover_compilers,
    discover_gmod_root,
    executable_format,
    host_compatibility,
    inspect_bsp_artifact,
    inspect_prt_artifact,
    parse_compiler_log,
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


def test_compile_invocation_plan_builds_stage_commands(tmp_path: Path) -> None:
    vbsp = tmp_path / "vbsp"
    vvis = tmp_path / "vvis"
    vrad = tmp_path / "vrad"
    vmf = tmp_path / "generated.vmf"
    workdir = tmp_path / "work"
    for executable in (vbsp, vvis, vrad):
        executable.write_bytes(b"\x7fELFpayload")
    vmf.write_text("world\n{\n}\n", encoding="utf-8")
    preflight = build_compiler_run_preflight(
        CompilerSet(vbsp=vbsp, vvis=vvis, vrad=vrad, bspzip=None), host="Linux"
    )

    plan = build_compile_invocation_plan(
        preflight,
        source_vmf=vmf,
        workdir=workdir,
        map_name="generated",
    )

    assert plan.status is CompilerInvocationStatus.READY
    assert plan.blockers == ()
    assert plan.bsp_path == workdir / "generated.bsp"
    assert [(command.role, command.argv) for command in plan.commands] == [
        ("vbsp", (str(vbsp), str(vmf))),
        ("vvis", (str(vvis), str(workdir / "generated.bsp"))),
        ("vrad", (str(vrad), str(workdir / "generated.bsp"))),
    ]
    assert [command.stdout_path.name for command in plan.commands] == [
        "01-vbsp.stdout.log",
        "02-vvis.stdout.log",
        "03-vrad.stdout.log",
    ]


def test_compile_invocation_plan_prefixes_compatibility_runner(tmp_path: Path) -> None:
    vbsp = tmp_path / "vbsp.exe"
    vvis = tmp_path / "vvis.exe"
    vrad = tmp_path / "vrad.exe"
    vmf = tmp_path / "generated.vmf"
    workdir = tmp_path / "work"
    for executable in (vbsp, vvis, vrad):
        executable.write_bytes(b"MZpayload")
    vmf.write_text("world\n{\n}\n", encoding="utf-8")
    preflight = CompilerRunPreflight(
        status=CompilerRunStatus.READY,
        tools=(
            CompilerRunTool("vbsp", vbsp, "windows-pe", "compatibility-layer-required"),
            CompilerRunTool("vvis", vvis, "windows-pe", "compatibility-layer-required"),
            CompilerRunTool("vrad", vrad, "windows-pe", "compatibility-layer-required"),
        ),
        blockers=(),
        runner_command="wine64",
    )

    plan = build_compile_invocation_plan(
        preflight,
        source_vmf=vmf,
        workdir=workdir,
        map_name="generated",
    )

    assert plan.status is CompilerInvocationStatus.READY
    assert plan.commands[0].argv == ("wine64", str(vbsp), str(vmf))


def test_compile_invocation_plan_blocks_unready_inputs(tmp_path: Path) -> None:
    plan = build_compile_invocation_plan(
        CompilerRunPreflight(
            status=CompilerRunStatus.BLOCKED,
            tools=(),
            blockers=(),
        ),
        source_vmf=tmp_path / "missing.vmf",
        workdir=tmp_path / "work",
        map_name="",
    )

    assert plan.status is CompilerInvocationStatus.BLOCKED
    assert plan.commands == ()
    assert [blocker.code for blocker in plan.blockers] == [
        CompilerInvocationBlockerCode.PREFLIGHT_BLOCKED,
        CompilerInvocationBlockerCode.SOURCE_VMF_MISSING,
        CompilerInvocationBlockerCode.EMPTY_MAP_NAME,
    ]


def test_compiler_log_parser_reports_clean_statistics() -> None:
    report = parse_compiler_log(
        """Valve Software - vbsp.exe
numportals: 128
numareas: 4
writing c:\\maps\\generated.bsp
"""
    )

    assert report.status is CompilerLogStatus.CLEAN
    assert report.blocking_message_count == 0
    assert [(message.code, message.blocking) for message in report.messages] == [
        (CompilerLogMessageCode.PORTAL_STATISTIC, False),
        (CompilerLogMessageCode.AREA_STATISTIC, False),
    ]


def test_compiler_log_parser_blocks_leaks_limits_and_fatal_errors() -> None:
    report = parse_compiler_log(
        """**** leaked ****
Too many T-junctions to fix up!
Error: displacement found on a(n) func_detail entity - not supported
"""
    )

    assert report.status is CompilerLogStatus.BLOCKED
    assert report.blocking_message_count == 3
    assert [message.code for message in report.messages] == [
        CompilerLogMessageCode.LEAK_DETECTED,
        CompilerLogMessageCode.LIMIT_EXCEEDED,
        CompilerLogMessageCode.FATAL_ERROR,
    ]


def test_compiler_log_parser_blocks_unknown_error_like_lines() -> None:
    report = parse_compiler_log("unexpected frobnicator error near portal 12\n")

    assert report.status is CompilerLogStatus.BLOCKED
    assert [(message.code, message.line_number, message.raw) for message in report.messages] == [
        (
            CompilerLogMessageCode.UNKNOWN_ERROR_LIKE_OUTPUT,
            1,
            "unexpected frobnicator error near portal 12",
        )
    ]


def test_bsp_artifact_inspection_accepts_valid_header_and_lump_lengths(tmp_path: Path) -> None:
    bsp = tmp_path / "generated.bsp"
    bsp.write_bytes(_bsp_header(version=21, lump_lengths={0: 128, 1: 256}))

    report = inspect_bsp_artifact(bsp)

    assert report.status is CompilerArtifactStatus.VALID
    assert report.blockers == ()
    assert report.bsp_version == 21
    assert report.lump_lengths[0] == 128
    assert report.lump_lengths[1] == 256


def test_bsp_artifact_inspection_blocks_missing_short_and_bad_magic(tmp_path: Path) -> None:
    missing = inspect_bsp_artifact(tmp_path / "missing.bsp")
    assert missing.status is CompilerArtifactStatus.BLOCKED
    assert [blocker.code for blocker in missing.blockers] == [
        CompilerArtifactBlockerCode.BSP_MISSING
    ]

    short = tmp_path / "short.bsp"
    short.write_bytes(b"VBSP")
    short_report = inspect_bsp_artifact(short)
    assert [blocker.code for blocker in short_report.blockers] == [
        CompilerArtifactBlockerCode.BSP_HEADER_TRUNCATED
    ]

    bad_magic = tmp_path / "bad.bsp"
    bad_magic.write_bytes(b"NOPE" + b"\x00" * 1032)
    bad_magic_report = inspect_bsp_artifact(bad_magic)
    assert [blocker.code for blocker in bad_magic_report.blockers] == [
        CompilerArtifactBlockerCode.BSP_BAD_MAGIC
    ]


def test_prt_artifact_inspection_reads_portal_and_leaf_counts(tmp_path: Path) -> None:
    prt = tmp_path / "generated.prt"
    prt.write_text("PRT1\n4\n128\n", encoding="ascii")

    report = inspect_prt_artifact(prt)

    assert report.status is CompilerArtifactStatus.VALID
    assert report.blockers == ()
    assert report.portal_count == 128
    assert report.leaf_count == 4


def test_prt_artifact_inspection_blocks_missing_or_malformed_files(tmp_path: Path) -> None:
    missing = inspect_prt_artifact(tmp_path / "missing.prt")
    assert [blocker.code for blocker in missing.blockers] == [
        CompilerArtifactBlockerCode.PRT_MISSING
    ]

    malformed = tmp_path / "generated.prt"
    malformed.write_text("PRT1\nnot-a-number\n", encoding="ascii")
    malformed_report = inspect_prt_artifact(malformed)
    assert [blocker.code for blocker in malformed_report.blockers] == [
        CompilerArtifactBlockerCode.PRT_MALFORMED
    ]


def _bsp_header(version: int, lump_lengths: dict[int, int]) -> bytes:
    import struct

    lumps = bytearray()
    for index in range(64):
        lumps.extend(struct.pack("<iii4s", 0, lump_lengths.get(index, 0), 0, b"\x00" * 4))
    return b"VBSP" + struct.pack("<i", version) + bytes(lumps) + struct.pack("<i", 1)
