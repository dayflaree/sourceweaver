"""Stable report and provenance models."""

from __future__ import annotations

from datetime import UTC, datetime
from enum import StrEnum
from pathlib import Path
from typing import Any

from pydantic import BaseModel, ConfigDict, Field


class Severity(StrEnum):
    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    BLOCKER = "blocker"


class Confidence(StrEnum):
    PROVEN = "proven"
    HIGH = "high"
    MEDIUM = "medium"
    LOW = "low"
    UNKNOWN = "unknown"


class Diagnostic(BaseModel):
    model_config = ConfigDict(frozen=True)

    code: str
    severity: Severity
    message: str
    evidence: list[str] = Field(default_factory=list)
    remediation: str | None = None
    confidence: Confidence = Confidence.PROVEN
    object_refs: list[str] = Field(default_factory=list)


class ArtifactFingerprint(BaseModel):
    model_config = ConfigDict(frozen=True)

    path: str
    size: int
    sha256: str

    @classmethod
    def from_path(cls, path: Path) -> ArtifactFingerprint:
        import hashlib

        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        return cls(path=str(path), size=path.stat().st_size, sha256=digest.hexdigest())


class AnalysisReport(BaseModel):
    model_config = ConfigDict(frozen=True)

    schema_version: str = "1.0"
    generated_at: datetime = Field(default_factory=lambda: datetime.now(UTC))
    source: ArtifactFingerprint
    encoding: str
    newline: str
    top_level_blocks: list[str]
    diagnostics: list[Diagnostic] = Field(default_factory=list)
    metadata: dict[str, Any] = Field(default_factory=dict)
