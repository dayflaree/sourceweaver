"""Compiler discovery and immutable fingerprinting.

This module deliberately does not infer hard limits from a different Source
branch. A profile is tied to the exact executable hashes used for validation.
"""

from __future__ import annotations

import os
import shutil
from dataclasses import dataclass
from pathlib import Path

from sourceweaver.model import ArtifactFingerprint


@dataclass(frozen=True, slots=True)
class CompilerSet:
    vbsp: Path | None
    vvis: Path | None
    vrad: Path | None
    bspzip: Path | None

    def fingerprints(self) -> dict[str, ArtifactFingerprint | None]:
        return {
            "vbsp": ArtifactFingerprint.from_path(self.vbsp) if self.vbsp else None,
            "vvis": ArtifactFingerprint.from_path(self.vvis) if self.vvis else None,
            "vrad": ArtifactFingerprint.from_path(self.vrad) if self.vrad else None,
            "bspzip": ArtifactFingerprint.from_path(self.bspzip) if self.bspzip else None,
        }


def _first_existing(candidates: list[Path]) -> Path | None:
    return next((candidate for candidate in candidates if candidate.is_file()), None)


def discover_gmod_root(explicit: str | Path | None = None) -> Path | None:
    """Find a Garry's Mod installation using explicit and common paths."""
    candidates: list[Path] = []
    if explicit:
        candidates.append(Path(explicit).expanduser())
    if env_root := os.environ.get("SOURCEWEAVER_GMOD_ROOT"):
        candidates.append(Path(env_root).expanduser())
    home = Path.home()
    candidates.extend(
        [
            home / ".local/share/Steam/steamapps/common/GarrysMod",
            home / ".steam/steam/steamapps/common/GarrysMod",
            home / "snap/steam/common/.local/share/Steam/steamapps/common/GarrysMod",
            Path("C:/Program Files (x86)/Steam/steamapps/common/GarrysMod"),
        ]
    )
    return next((candidate for candidate in candidates if candidate.is_dir()), None)


def discover_compilers(gmod_root: str | Path) -> CompilerSet:
    """Discover current Tools++ first, then stock compiler names."""
    root = Path(gmod_root)
    dirs = [root / "bin/win64", root / "bin/x64", root / "bin"]

    def find(*names: str) -> Path | None:
        candidates = [directory / name for directory in dirs for name in names]
        found = _first_existing(candidates)
        if found:
            return found
        for name in names:
            executable = shutil.which(name)
            if executable:
                return Path(executable)
        return None

    return CompilerSet(
        vbsp=find("vbspplusplus.exe", "vbsp.exe", "vbsp"),
        vvis=find("vvisplusplus.exe", "vvis.exe", "vvis"),
        vrad=find("vradplusplus.exe", "vrad.exe", "vrad"),
        bspzip=find("bspzipplusplus.exe", "bspzip.exe", "bspzip"),
    )
