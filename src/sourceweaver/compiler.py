"""Cross-platform compiler discovery and immutable fingerprinting.

This module deliberately does not infer hard limits from a different Source
branch. A profile is tied to the exact executable hashes used for validation.
"""

from __future__ import annotations

import hashlib
import importlib
import os
import platform
import re
import shutil
import struct
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Final

from sourceweaver.model import ArtifactFingerprint

_COMPILER_NAMES: Final[dict[str, tuple[str, ...]]] = {
    "vbsp": ("vbspplusplus.exe", "vbsp.exe", "vbsp"),
    "vvis": ("vvisplusplus.exe", "vvis.exe", "vvis"),
    "vrad": ("vradplusplus.exe", "vrad.exe", "vrad"),
    "bspzip": ("bspzipplusplus.exe", "bspzip.exe", "bspzip"),
}
_VDF_PATH_RE: Final[re.Pattern[str]] = re.compile(
    r'"path"\s*"(?P<path>(?:\\.|[^"\\])*)"', re.IGNORECASE
)
_VDF_LEGACY_LIBRARY_RE: Final[re.Pattern[str]] = re.compile(
    r'^\s*"\d+"\s*"(?P<path>(?:\\.|[^"\\])*)"\s*$', re.MULTILINE
)
_REQUIRED_COMPILE_ROLES: Final[tuple[str, ...]] = ("vbsp", "vvis", "vrad")
_COMPATIBILITY_RUNNERS: Final[tuple[str, ...]] = ("wine64", "wine")
_SOURCE_BSP_HEADER_SIZE: Final[int] = 4 + 4 + (64 * 16) + 4


class CompilerRunStatus(StrEnum):
    """Final status for compiler invocation readiness."""

    READY = "ready"
    BLOCKED = "blocked"


class CompilerRunBlockerCode(StrEnum):
    """Deterministic blockers for compiler invocation readiness."""

    MISSING_COMPILER = "missing_compiler"
    COMPATIBILITY_LAYER_REQUIRED = "compatibility_layer_required"
    UNSUPPORTED_FORMAT = "unsupported_format"
    UNKNOWN_HOST = "unknown_host"


class CompilerInvocationStatus(StrEnum):
    """Final status for compiler invocation command planning."""

    READY = "ready"
    BLOCKED = "blocked"


class CompilerInvocationBlockerCode(StrEnum):
    """Deterministic blockers for compiler invocation command planning."""

    PREFLIGHT_BLOCKED = "preflight_blocked"
    SOURCE_VMF_MISSING = "source_vmf_missing"
    EMPTY_MAP_NAME = "empty_map_name"
    REQUIRED_STAGE_MISSING = "required_stage_missing"


class CompilerLogStatus(StrEnum):
    """Final status for normalized compiler log parsing."""

    CLEAN = "clean"
    BLOCKED = "blocked"


class CompilerLogMessageCode(StrEnum):
    """Normalized compiler log message classes."""

    LEAK_DETECTED = "leak_detected"
    LIMIT_EXCEEDED = "limit_exceeded"
    FATAL_ERROR = "fatal_error"
    UNKNOWN_ERROR_LIKE_OUTPUT = "unknown_error_like_output"
    PORTAL_STATISTIC = "portal_statistic"
    AREA_STATISTIC = "area_statistic"


class CompilerArtifactStatus(StrEnum):
    """Final status for compiler artifact inspection."""

    VALID = "valid"
    BLOCKED = "blocked"


class CompilerArtifactBlockerCode(StrEnum):
    """Deterministic blockers for compiler artifact inspection."""

    BSP_MISSING = "bsp_missing"
    BSP_HEADER_TRUNCATED = "bsp_header_truncated"
    BSP_BAD_MAGIC = "bsp_bad_magic"
    PRT_MISSING = "prt_missing"
    PRT_MALFORMED = "prt_malformed"


class CompilerWorktreeStatus(StrEnum):
    """Final status for content-addressed compiler worktree planning."""

    READY = "ready"
    BLOCKED = "blocked"


class CompilerWorktreeBlockerCode(StrEnum):
    """Deterministic blockers for compiler worktree planning."""

    PREFLIGHT_BLOCKED = "preflight_blocked"
    SOURCE_VMF_MISSING = "source_vmf_missing"
    EMPTY_PROFILE_NAME = "empty_profile_name"
    EMPTY_MAP_NAME = "empty_map_name"


@dataclass(frozen=True, slots=True)
class CompilerSet:
    vbsp: Path | None
    vvis: Path | None
    vrad: Path | None
    bspzip: Path | None

    def items(self) -> tuple[tuple[str, Path | None], ...]:
        """Return compilers in pipeline order."""
        return (
            ("vbsp", self.vbsp),
            ("vvis", self.vvis),
            ("vrad", self.vrad),
            ("bspzip", self.bspzip),
        )

    @property
    def complete(self) -> bool:
        """Whether every expected compiler was discovered."""
        return all(path is not None for _, path in self.items())

    def fingerprints(self) -> dict[str, ArtifactFingerprint | None]:
        return {
            name: ArtifactFingerprint.from_path(path) if path else None
            for name, path in self.items()
        }


@dataclass(frozen=True, slots=True)
class CompilerRunTool:
    """One compiler executable enrolled for an invocation plan."""

    role: str
    path: Path
    executable_format: str
    compatibility: str


@dataclass(frozen=True, slots=True)
class CompilerRunBlocker:
    """A reason the selected compiler set cannot be invoked."""

    code: CompilerRunBlockerCode
    message: str
    role: str
    path: Path | None = None


@dataclass(frozen=True, slots=True)
class CompilerRunPreflight:
    """Read-only compiler invocation readiness report."""

    status: CompilerRunStatus
    tools: tuple[CompilerRunTool, ...]
    blockers: tuple[CompilerRunBlocker, ...]
    runner_command: str | None = None


@dataclass(frozen=True, slots=True)
class CompilerStageCommand:
    """One argument-array compiler stage command."""

    role: str
    argv: tuple[str, ...]
    workdir: Path
    stdout_path: Path
    stderr_path: Path


@dataclass(frozen=True, slots=True)
class CompilerInvocationBlocker:
    """A reason compiler command planning cannot proceed."""

    code: CompilerInvocationBlockerCode
    message: str
    role: str | None = None
    path: Path | None = None


@dataclass(frozen=True, slots=True)
class CompilerInvocationPlan:
    """Read-only compiler invocation command plan."""

    status: CompilerInvocationStatus
    source_vmf: Path
    workdir: Path
    bsp_path: Path
    commands: tuple[CompilerStageCommand, ...]
    blockers: tuple[CompilerInvocationBlocker, ...]


@dataclass(frozen=True, slots=True)
class CompilerLogMessage:
    """One normalized compiler log message."""

    code: CompilerLogMessageCode
    line_number: int
    raw: str
    blocking: bool


@dataclass(frozen=True, slots=True)
class CompilerLogReport:
    """Normalized compiler log parse report."""

    status: CompilerLogStatus
    messages: tuple[CompilerLogMessage, ...]
    blocking_message_count: int


@dataclass(frozen=True, slots=True)
class CompilerArtifactBlocker:
    """A reason compiler artifact inspection failed."""

    code: CompilerArtifactBlockerCode
    message: str
    path: Path


@dataclass(frozen=True, slots=True)
class BspArtifactReport:
    """Minimal Source BSP artifact inspection report."""

    status: CompilerArtifactStatus
    path: Path
    bsp_version: int | None
    lump_lengths: tuple[int, ...]
    blockers: tuple[CompilerArtifactBlocker, ...]


@dataclass(frozen=True, slots=True)
class PrtArtifactReport:
    """Minimal Source PRT artifact inspection report."""

    status: CompilerArtifactStatus
    path: Path
    leaf_count: int | None
    portal_count: int | None
    blockers: tuple[CompilerArtifactBlocker, ...]


@dataclass(frozen=True, slots=True)
class CompilerWorktreeBlocker:
    """A reason a compiler worktree layout cannot be planned."""

    code: CompilerWorktreeBlockerCode
    message: str
    path: Path | None = None


@dataclass(frozen=True, slots=True)
class CompilerWorktreeLayout:
    """Content-addressed worktree layout for a compile attempt."""

    status: CompilerWorktreeStatus
    cache_key: str
    profile_name: str
    map_name: str
    cache_root: Path
    workdir: Path
    source_vmf: Path
    source_vmf_copy: Path
    expected_bsp: Path
    log_dir: Path
    manifest_path: Path
    blockers: tuple[CompilerWorktreeBlocker, ...]


def _decode_vdf_string(value: str) -> str:
    output: list[str] = []
    escaped = False
    for char in value:
        if escaped:
            if char in {'"', "\\"}:
                output.append(char)
            else:
                output.extend(("\\", char))
            escaped = False
        elif char == "\\":
            escaped = True
        else:
            output.append(char)
    if escaped:
        output.append("\\")
    return "".join(output)


def parse_steam_library_paths(text: str) -> tuple[Path, ...]:
    """Extract Steam library roots from modern or legacy libraryfolders VDF."""
    paths: list[Path] = []
    seen: set[str] = set()
    matches = (*_VDF_PATH_RE.finditer(text), *_VDF_LEGACY_LIBRARY_RE.finditer(text))
    for match in matches:
        value = _decode_vdf_string(match.group("path"))
        key = os.path.normcase(os.path.normpath(value))
        if key not in seen:
            seen.add(key)
            paths.append(Path(value))
    return tuple(paths)


def _registry_steam_roots() -> tuple[Path, ...]:
    """Read Steam installation roots from the Windows registry when available."""
    if os.name != "nt":
        return ()
    try:
        winreg = importlib.import_module("winreg")
    except ImportError:
        return ()

    roots: list[Path] = []
    locations = (
        (winreg.HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
        (winreg.HKEY_LOCAL_MACHINE, r"Software\WOW6432Node\Valve\Steam", "InstallPath"),
        (winreg.HKEY_LOCAL_MACHINE, r"Software\Valve\Steam", "InstallPath"),
    )
    for hive, key_name, value_name in locations:
        try:
            with winreg.OpenKey(hive, key_name) as key:
                value, _ = winreg.QueryValueEx(key, value_name)
        except OSError:
            continue
        if isinstance(value, str) and value:
            roots.append(Path(value))
    return tuple(roots)


def _default_steam_roots(*, home: Path, environ: Mapping[str, str]) -> tuple[Path, ...]:
    roots: list[Path] = []
    for name in ("STEAM_DIR", "STEAM_PATH"):
        if value := environ.get(name):
            roots.append(Path(value).expanduser())

    roots.extend(
        [
            home / ".local" / "share" / "Steam",
            home / ".steam" / "steam",
            home / ".steam" / "debian-installation",
            home / ".var" / "app" / "com.valvesoftware.Steam" / ".local" / "share" / "Steam",
            home / "snap" / "steam" / "common" / ".local" / "share" / "Steam",
        ]
    )
    for name in ("ProgramFiles(x86)", "ProgramFiles"):
        if value := environ.get(name):
            roots.append(Path(value) / "Steam")
    roots.extend(_registry_steam_roots())
    return tuple(roots)


def _library_roots(steam_roots: Iterable[Path]) -> tuple[Path, ...]:
    libraries: list[Path] = []
    seen: set[str] = set()
    for steam_root in steam_roots:
        candidates = [steam_root]
        vdf = steam_root / "steamapps" / "libraryfolders.vdf"
        try:
            text = vdf.read_text(encoding="utf-8-sig", errors="replace")
        except OSError:
            pass
        else:
            candidates.extend(parse_steam_library_paths(text))

        for candidate in candidates:
            key = os.path.normcase(os.path.normpath(str(candidate.expanduser())))
            if key not in seen:
                seen.add(key)
                libraries.append(candidate.expanduser())
    return tuple(libraries)


def _normalize_gmod_root(candidate: Path) -> Path | None:
    expanded = candidate.expanduser()
    if not expanded.is_dir():
        return None
    if (expanded / "bin").is_dir() and (expanded / "garrysmod").is_dir():
        return expanded
    if (
        expanded.name.casefold() == "garrysmod"
        and (expanded / "gameinfo.txt").is_file()
        and (expanded.parent / "bin").is_dir()
    ):
        return expanded.parent
    return None


def discover_gmod_root(
    explicit: str | Path | None = None,
    *,
    environ: Mapping[str, str] | None = None,
    home: Path | None = None,
    steam_roots: Iterable[Path] | None = None,
) -> Path | None:
    """Find a Garry's Mod installation across Windows and Linux Steam libraries."""
    environment = os.environ if environ is None else environ
    user_home = Path.home() if home is None else home
    direct: list[Path] = []
    if explicit:
        direct.append(Path(explicit))
    if env_root := environment.get("SOURCEWEAVER_GMOD_ROOT"):
        direct.append(Path(env_root))

    roots = (
        tuple(steam_roots)
        if steam_roots is not None
        else _default_steam_roots(home=user_home, environ=environment)
    )
    candidates = direct + [
        library / "steamapps" / "common" / "GarrysMod" for library in _library_roots(roots)
    ]

    seen: set[str] = set()
    for candidate in candidates:
        key = os.path.normcase(os.path.abspath(os.path.expanduser(str(candidate))))
        if key in seen:
            continue
        seen.add(key)
        if normalized := _normalize_gmod_root(candidate):
            return normalized
    return None


def _first_existing(candidates: Iterable[Path]) -> Path | None:
    return next((candidate for candidate in candidates if candidate.is_file()), None)


def discover_compilers(gmod_root: str | Path, *, include_path: bool = False) -> CompilerSet:
    """Discover current Tools++ first, then stock compiler names.

    PATH fallback is disabled by default so an unrelated global compiler cannot
    silently replace the selected game's toolchain.
    """
    root = Path(gmod_root)
    directories = [root / "bin" / "win64", root / "bin" / "x64", root / "bin"]

    def find(names: tuple[str, ...]) -> Path | None:
        candidates = [directory / name for directory in directories for name in names]
        if found := _first_existing(candidates):
            return found
        if include_path:
            for name in names:
                if executable := shutil.which(name):
                    return Path(executable)
        return None

    return CompilerSet(**{role: find(names) for role, names in _COMPILER_NAMES.items()})


def executable_format(path: str | Path) -> str:
    """Identify the executable container from its magic bytes."""
    with Path(path).open("rb") as stream:
        magic = stream.read(4)
    if magic[:2] == b"MZ":
        return "windows-pe"
    if magic == b"\x7fELF":
        return "linux-elf"
    if magic[:4] in {b"\xcf\xfa\xed\xfe", b"\xfe\xed\xfa\xcf"}:
        return "macos-mach-o"
    return "unknown"


def host_compatibility(executable_kind: str, *, host: str | None = None) -> str:
    """Describe whether an executable is native to the current host."""
    system = (platform.system() if host is None else host).casefold()
    if system == "windows":
        return "native" if executable_kind == "windows-pe" else "unsupported-format"
    if system == "linux":
        if executable_kind == "linux-elf":
            return "native"
        if executable_kind == "windows-pe":
            return "compatibility-layer-required"
        return "unsupported-format"
    if system == "darwin":
        return "native" if executable_kind == "macos-mach-o" else "unsupported-format"
    return "unknown-host"


def build_compiler_run_preflight(
    compilers: CompilerSet,
    *,
    host: str | None = None,
    required_roles: Iterable[str] = _REQUIRED_COMPILE_ROLES,
    compatibility_runners: Iterable[str] = _COMPATIBILITY_RUNNERS,
) -> CompilerRunPreflight:
    """Report whether the selected compiler set can be invoked on this host."""
    role_to_path = dict(compilers.items())
    tools: list[CompilerRunTool] = []
    blockers: list[CompilerRunBlocker] = []
    needs_runner = False
    for role in required_roles:
        path = role_to_path.get(role)
        if path is None:
            blockers.append(
                CompilerRunBlocker(
                    code=CompilerRunBlockerCode.MISSING_COMPILER,
                    message="Required compiler executable was not discovered.",
                    role=role,
                )
            )
            continue
        executable_kind = executable_format(path)
        compatibility = host_compatibility(executable_kind, host=host)
        tools.append(
            CompilerRunTool(
                role=role,
                path=path,
                executable_format=executable_kind,
                compatibility=compatibility,
            )
        )
        if compatibility == "native":
            continue
        if compatibility == "compatibility-layer-required":
            needs_runner = True
            continue
        message = f"Compiler executable format is not runnable on this host: {compatibility}."
        blockers.append(
            CompilerRunBlocker(
                code=_compatibility_blocker_code(compatibility),
                message=message,
                role=role,
                path=path,
            )
        )

    runner = _first_available_runner(compatibility_runners) if needs_runner else None
    if needs_runner and runner is None:
        blockers.extend(
            CompilerRunBlocker(
                code=CompilerRunBlockerCode.COMPATIBILITY_LAYER_REQUIRED,
                message="Compiler requires a compatibility layer, but none was found.",
                role=tool.role,
                path=tool.path,
            )
            for tool in tools
            if tool.compatibility == "compatibility-layer-required"
        )
    return CompilerRunPreflight(
        status=CompilerRunStatus.BLOCKED if blockers else CompilerRunStatus.READY,
        tools=tuple(tools),
        blockers=tuple(blockers),
        runner_command=runner,
    )


def build_compile_invocation_plan(
    preflight: CompilerRunPreflight,
    *,
    source_vmf: Path,
    workdir: Path,
    map_name: str,
) -> CompilerInvocationPlan:
    """Build deterministic argument-array compiler commands without executing them."""
    bsp_path = workdir / f"{map_name}.bsp"
    blockers = _compile_invocation_blockers(
        preflight,
        source_vmf=source_vmf,
        map_name=map_name,
    )
    if blockers:
        return CompilerInvocationPlan(
            status=CompilerInvocationStatus.BLOCKED,
            source_vmf=source_vmf,
            workdir=workdir,
            bsp_path=bsp_path,
            commands=(),
            blockers=blockers,
        )
    tools_by_role = {tool.role: tool for tool in preflight.tools}
    missing_stage_blockers = tuple(
        CompilerInvocationBlocker(
            code=CompilerInvocationBlockerCode.REQUIRED_STAGE_MISSING,
            message="Required compiler stage is absent from the ready preflight.",
            role=role,
        )
        for role in _REQUIRED_COMPILE_ROLES
        if role not in tools_by_role
    )
    if missing_stage_blockers:
        return CompilerInvocationPlan(
            status=CompilerInvocationStatus.BLOCKED,
            source_vmf=source_vmf,
            workdir=workdir,
            bsp_path=bsp_path,
            commands=(),
            blockers=missing_stage_blockers,
        )
    return CompilerInvocationPlan(
        status=CompilerInvocationStatus.READY,
        source_vmf=source_vmf,
        workdir=workdir,
        bsp_path=bsp_path,
        commands=tuple(
            _stage_command(
                index=index,
                role=role,
                tool=tools_by_role[role],
                runner_command=preflight.runner_command,
                source_vmf=source_vmf,
                bsp_path=bsp_path,
                workdir=workdir,
            )
            for index, role in enumerate(_REQUIRED_COMPILE_ROLES, start=1)
        ),
        blockers=(),
    )


def build_compile_worktree_layout(
    preflight: CompilerRunPreflight,
    *,
    source_vmf: Path,
    cache_root: Path,
    profile_name: str,
    map_name: str,
) -> CompilerWorktreeLayout:
    """Build a content-addressed compiler worktree layout without writing files."""
    blockers = _compile_worktree_blockers(
        preflight,
        source_vmf=source_vmf,
        profile_name=profile_name,
        map_name=map_name,
    )
    cache_key = _compile_cache_key(preflight, source_vmf, profile_name, map_name)
    workdir = cache_root / profile_name / cache_key
    if blockers:
        return CompilerWorktreeLayout(
            status=CompilerWorktreeStatus.BLOCKED,
            cache_key=cache_key,
            profile_name=profile_name,
            map_name=map_name,
            cache_root=cache_root,
            workdir=workdir,
            source_vmf=source_vmf,
            source_vmf_copy=workdir / f"{map_name}.vmf",
            expected_bsp=workdir / f"{map_name}.bsp",
            log_dir=workdir / "logs",
            manifest_path=workdir / "compile-manifest.json",
            blockers=blockers,
        )
    return CompilerWorktreeLayout(
        status=CompilerWorktreeStatus.READY,
        cache_key=cache_key,
        profile_name=profile_name,
        map_name=map_name,
        cache_root=cache_root,
        workdir=workdir,
        source_vmf=source_vmf,
        source_vmf_copy=workdir / f"{map_name}.vmf",
        expected_bsp=workdir / f"{map_name}.bsp",
        log_dir=workdir / "logs",
        manifest_path=workdir / "compile-manifest.json",
        blockers=(),
    )


def _compile_worktree_blockers(
    preflight: CompilerRunPreflight,
    *,
    source_vmf: Path,
    profile_name: str,
    map_name: str,
) -> tuple[CompilerWorktreeBlocker, ...]:
    blockers: list[CompilerWorktreeBlocker] = []
    if preflight.status is not CompilerRunStatus.READY:
        blockers.append(
            CompilerWorktreeBlocker(
                code=CompilerWorktreeBlockerCode.PREFLIGHT_BLOCKED,
                message="Compiler run preflight is blocked.",
            )
        )
    if not source_vmf.is_file():
        blockers.append(
            CompilerWorktreeBlocker(
                code=CompilerWorktreeBlockerCode.SOURCE_VMF_MISSING,
                message="Source VMF for compilation is missing.",
                path=source_vmf,
            )
        )
    if not profile_name:
        blockers.append(
            CompilerWorktreeBlocker(
                code=CompilerWorktreeBlockerCode.EMPTY_PROFILE_NAME,
                message="Compiler profile name must not be empty.",
            )
        )
    if not map_name:
        blockers.append(
            CompilerWorktreeBlocker(
                code=CompilerWorktreeBlockerCode.EMPTY_MAP_NAME,
                message="Compiler map name must not be empty.",
            )
        )
    return tuple(blockers)


def _compile_cache_key(
    preflight: CompilerRunPreflight,
    source_vmf: Path,
    profile_name: str,
    map_name: str,
) -> str:
    digest = hashlib.sha256()
    digest.update(profile_name.encode("utf-8"))
    digest.update(b"\0")
    digest.update(map_name.encode("utf-8"))
    digest.update(b"\0")
    if source_vmf.is_file():
        digest.update(source_vmf.read_bytes())
    digest.update(b"\0")
    for tool in preflight.tools:
        digest.update(tool.role.encode("utf-8"))
        digest.update(b"\0")
        digest.update(str(tool.path).encode("utf-8"))
        digest.update(b"\0")
        digest.update(tool.executable_format.encode("utf-8"))
        digest.update(b"\0")
        if tool.path.is_file():
            digest.update(ArtifactFingerprint.from_path(tool.path).sha256.encode("ascii"))
        digest.update(b"\0")
    runner = preflight.runner_command or ""
    digest.update(runner.encode("utf-8"))
    return digest.hexdigest()


def parse_compiler_log(text: str) -> CompilerLogReport:
    """Parse compiler stdout/stderr into normalized message classes."""
    messages: list[CompilerLogMessage] = []
    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        raw = raw_line.strip()
        if not raw:
            continue
        message = _classify_compiler_log_line(line_number, raw)
        if message is not None:
            messages.append(message)
    blocking_message_count = sum(1 for message in messages if message.blocking)
    return CompilerLogReport(
        status=CompilerLogStatus.BLOCKED if blocking_message_count else CompilerLogStatus.CLEAN,
        messages=tuple(messages),
        blocking_message_count=blocking_message_count,
    )


def inspect_bsp_artifact(path: Path) -> BspArtifactReport:
    """Inspect the Source BSP header without trusting compiler exit status."""
    if not path.is_file():
        return BspArtifactReport(
            status=CompilerArtifactStatus.BLOCKED,
            path=path,
            bsp_version=None,
            lump_lengths=(),
            blockers=(
                CompilerArtifactBlocker(
                    code=CompilerArtifactBlockerCode.BSP_MISSING,
                    message="Compiled BSP artifact is missing.",
                    path=path,
                ),
            ),
        )
    data = path.read_bytes()
    if len(data) < _SOURCE_BSP_HEADER_SIZE:
        return BspArtifactReport(
            status=CompilerArtifactStatus.BLOCKED,
            path=path,
            bsp_version=None,
            lump_lengths=(),
            blockers=(
                CompilerArtifactBlocker(
                    code=CompilerArtifactBlockerCode.BSP_HEADER_TRUNCATED,
                    message="Compiled BSP header is truncated.",
                    path=path,
                ),
            ),
        )
    if data[:4] != b"VBSP":
        return BspArtifactReport(
            status=CompilerArtifactStatus.BLOCKED,
            path=path,
            bsp_version=None,
            lump_lengths=(),
            blockers=(
                CompilerArtifactBlocker(
                    code=CompilerArtifactBlockerCode.BSP_BAD_MAGIC,
                    message="Compiled BSP artifact does not start with VBSP magic.",
                    path=path,
                ),
            ),
        )
    bsp_version = struct.unpack_from("<i", data, 4)[0]
    lump_lengths = tuple(
        struct.unpack_from("<i", data, 8 + (index * 16) + 4)[0] for index in range(64)
    )
    return BspArtifactReport(
        status=CompilerArtifactStatus.VALID,
        path=path,
        bsp_version=bsp_version,
        lump_lengths=lump_lengths,
        blockers=(),
    )


def inspect_prt_artifact(path: Path) -> PrtArtifactReport:
    """Inspect a Source PRT portal file's top-level counts."""
    if not path.is_file():
        return PrtArtifactReport(
            status=CompilerArtifactStatus.BLOCKED,
            path=path,
            leaf_count=None,
            portal_count=None,
            blockers=(
                CompilerArtifactBlocker(
                    code=CompilerArtifactBlockerCode.PRT_MISSING,
                    message="Compiled PRT artifact is missing.",
                    path=path,
                ),
            ),
        )
    try:
        lines = path.read_text(encoding="ascii", errors="strict").splitlines()
        leaf_count = int(lines[1].strip())
        portal_count = int(lines[2].strip())
    except (IndexError, UnicodeDecodeError, ValueError):
        return PrtArtifactReport(
            status=CompilerArtifactStatus.BLOCKED,
            path=path,
            leaf_count=None,
            portal_count=None,
            blockers=(
                CompilerArtifactBlocker(
                    code=CompilerArtifactBlockerCode.PRT_MALFORMED,
                    message="Compiled PRT artifact is malformed.",
                    path=path,
                ),
            ),
        )
    return PrtArtifactReport(
        status=CompilerArtifactStatus.VALID,
        path=path,
        leaf_count=leaf_count,
        portal_count=portal_count,
        blockers=(),
    )


def _classify_compiler_log_line(line_number: int, raw: str) -> CompilerLogMessage | None:
    normalized = raw.casefold()
    if "leaked" in normalized:
        return _compiler_log_message(
            CompilerLogMessageCode.LEAK_DETECTED, line_number, raw, blocking=True
        )
    if "too many" in normalized or "limit" in normalized or "max_" in normalized:
        return _compiler_log_message(
            CompilerLogMessageCode.LIMIT_EXCEEDED, line_number, raw, blocking=True
        )
    if normalized.startswith("error:") or "fatal" in normalized:
        return _compiler_log_message(
            CompilerLogMessageCode.FATAL_ERROR, line_number, raw, blocking=True
        )
    if "error" in normalized or "warning:" in normalized:
        return _compiler_log_message(
            CompilerLogMessageCode.UNKNOWN_ERROR_LIKE_OUTPUT,
            line_number,
            raw,
            blocking=True,
        )
    if normalized.startswith("numportals:"):
        return _compiler_log_message(
            CompilerLogMessageCode.PORTAL_STATISTIC, line_number, raw, blocking=False
        )
    if normalized.startswith("numareas:"):
        return _compiler_log_message(
            CompilerLogMessageCode.AREA_STATISTIC, line_number, raw, blocking=False
        )
    return None


def _compiler_log_message(
    code: CompilerLogMessageCode,
    line_number: int,
    raw: str,
    *,
    blocking: bool,
) -> CompilerLogMessage:
    return CompilerLogMessage(
        code=code,
        line_number=line_number,
        raw=raw,
        blocking=blocking,
    )


def _compile_invocation_blockers(
    preflight: CompilerRunPreflight,
    *,
    source_vmf: Path,
    map_name: str,
) -> tuple[CompilerInvocationBlocker, ...]:
    blockers: list[CompilerInvocationBlocker] = []
    if preflight.status is not CompilerRunStatus.READY:
        blockers.append(
            CompilerInvocationBlocker(
                code=CompilerInvocationBlockerCode.PREFLIGHT_BLOCKED,
                message="Compiler run preflight is blocked.",
            )
        )
    if not source_vmf.is_file():
        blockers.append(
            CompilerInvocationBlocker(
                code=CompilerInvocationBlockerCode.SOURCE_VMF_MISSING,
                message="Source VMF for compilation is missing.",
                path=source_vmf,
            )
        )
    if not map_name:
        blockers.append(
            CompilerInvocationBlocker(
                code=CompilerInvocationBlockerCode.EMPTY_MAP_NAME,
                message="Compiler invocation map name must not be empty.",
            )
        )
    return tuple(blockers)


def _stage_command(
    *,
    index: int,
    role: str,
    tool: CompilerRunTool,
    runner_command: str | None,
    source_vmf: Path,
    bsp_path: Path,
    workdir: Path,
) -> CompilerStageCommand:
    input_path = source_vmf if role == "vbsp" else bsp_path
    executable = str(tool.path)
    argv: tuple[str, ...] = (executable, str(input_path))
    if runner_command is not None:
        argv = (runner_command, *argv)
    return CompilerStageCommand(
        role=role,
        argv=argv,
        workdir=workdir,
        stdout_path=workdir / f"{index:02d}-{role}.stdout.log",
        stderr_path=workdir / f"{index:02d}-{role}.stderr.log",
    )


def _first_available_runner(runners: Iterable[str]) -> str | None:
    for runner in runners:
        if shutil.which(runner):
            return runner
    return None


def _compatibility_blocker_code(compatibility: str) -> CompilerRunBlockerCode:
    if compatibility == "unknown-host":
        return CompilerRunBlockerCode.UNKNOWN_HOST
    return CompilerRunBlockerCode.UNSUPPORTED_FORMAT
