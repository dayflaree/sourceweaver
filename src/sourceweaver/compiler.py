"""Cross-platform compiler discovery and immutable fingerprinting.

This module deliberately does not infer hard limits from a different Source
branch. A profile is tied to the exact executable hashes used for validation.
"""

from __future__ import annotations

import importlib
import os
import platform
import re
import shutil
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


def _first_available_runner(runners: Iterable[str]) -> str | None:
    for runner in runners:
        if shutil.which(runner):
            return runner
    return None


def _compatibility_blocker_code(compatibility: str) -> CompilerRunBlockerCode:
    if compatibility == "unknown-host":
        return CompilerRunBlockerCode.UNKNOWN_HOST
    return CompilerRunBlockerCode.UNSUPPORTED_FORMAT
