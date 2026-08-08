#!/usr/bin/env python3
"""Guard release/docs wording against unsupported compatibility claims."""
from __future__ import annotations

import argparse
import fnmatch
import json
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ALLOWLIST = ROOT / "docs" / "validation-claim-allowlist.json"

SCAN_GLOBS = [
    "README.md",
    "CHANGELOG.md",
    "docs/**/*.md",
    "release-notes/**/*.md",
    "docs/release-notes/**/*.md",
    ".github/release-notes/**/*.md",
    "RELEASE_NOTES*.md",
]

SKIP_DIRS = {".git", "target", "node_modules", ".hermes"}

SAFE_CONTEXT = re.compile(
    r"\b("
    r"not|no|never|without|unless|until|requires?|requirement|future|planned|when configured|if configured|can verify|"
    r"separate|failure|failed|blocked|unvalidated|uncertified|not certified|not enabled|does not|did not|"
    r"cannot|only after|out of scope|manual|opt[- ]in|handoff|does not claim|do not claim|must not claim|"
    r"not yet|absent|refuse|refused|unsupported|limitation|limitations|evidence requirements?"
    r")\b",
    re.IGNORECASE,
)


@dataclass(frozen=True)
class Rule:
    category: str
    description: str
    patterns: tuple[re.Pattern[str], ...]


RULES: tuple[Rule, ...] = (
    Rule(
        "hammer-open-save",
        "Hammer/Hammer++ compatibility or certification claim",
        (
            re.compile(r"\bHammer(?:\+\+)?(?:\s+open/save)?\s+(?:compatible|certified|validated|proven|verified|ready)\b", re.I),
            re.compile(r"\bcompatible\s+with\s+Hammer(?:\+\+)?\b", re.I),
            re.compile(r"\bHammer(?:\+\+)?[- ]compatible\b", re.I),
            re.compile(r"\bHammer(?:\+\+)?\s+open/save\s+(?:passes|passed|validated|certified|complete|completed)\b", re.I),
        ),
    ),
    Rule(
        "native-windows-compiler",
        "native Windows Source compiler validation claim",
        (
            re.compile(r"\bnative\s+Windows\s+(?:Source\s+)?(?:compiler|VBSP|VVIS|VRAD)[^\n.]*\b(?:validated|certified|passes|passed|works|supported|ready|complete|completed)\b", re.I),
            re.compile(r"\bvalidated\s+native\s+Windows\s+(?:Source\s+)?(?:compiler|VBSP|VVIS|VRAD)\b", re.I),
        ),
    ),
    Rule(
        "game-runtime-load",
        "successful game-runtime map-load claim",
        (
            re.compile(r"\b(?:game[- ]runtime|game\s+runtime|runtime)\s+(?:map[- ]load|load)[^\n.]*\b(?:passed|passes|succeeded|successful|validated|certified|complete|completed)\b", re.I),
            re.compile(r"\bsuccessful\s+(?:real\s+)?(?:game[- ]runtime|game\s+runtime)[^\n.]*\b(?:map[- ]load|load)\b", re.I),
            re.compile(r"\bgame[- ]playable\s+map[- ]load\s+pass\b", re.I),
        ),
    ),
    Rule(
        "rendered-hlmv-preview",
        "rendered HLMV/HLMV++ model preview success claim",
        (
            re.compile(r"\brendered\s+HLMV(?:\+\+)?(?:\s+model)?\s+preview[^\n.]*\b(?:passed|passes|succeeded|successful|validated|certified|complete|completed)\b", re.I),
            re.compile(r"\bHLMV(?:\+\+)?[^\n.]*\brendered\s+(?:window|preview)[^\n.]*\b(?:passed|passes|succeeded|successful|validated|certified|complete|completed)\b", re.I),
        ),
    ),
    Rule(
        "signed-windows-release",
        "signed Windows release/artifact claim",
        (
            re.compile(r"\bsigned\s+Windows\s+(?:release|build|installer|artifact|setup)\b", re.I),
            re.compile(r"\bWindows\s+(?:release|build|installer|artifact|setup)[^\n.]*\b(?:is|are|was|were)\s+signed\b", re.I),
        ),
    ),
    Rule(
        "automatic-updates",
        "automatic update/install/rollback claim",
        (
            re.compile(r"\bautomatic\s+(?:self[- ])?updates?[^\n.]*\b(?:enabled|implemented|supported|available|on)\b", re.I),
            re.compile(r"\bauto(?:matic)?\s+(?:installer\s+execution|install|self[- ]update|rollback|executable\s+replacement)[^\n.]*\b(?:enabled|implemented|supported|available)\b", re.I),
            re.compile(r"\bsilent\s+install[^\n.]*\b(?:enabled|implemented|supported|available)\b", re.I),
        ),
    ),
    Rule(
        "textured-hammer-equivalent-preview",
        "textured Hammer-equivalent preview claim",
        (
            re.compile(r"\b(?:textured\s+Hammer(?:-equivalent)?\s+preview|Hammer-equivalent\s+textured\s+viewport|full\s+textured\s+Hammer\s+clone|exact\s+Hammer\s+UV\s+projection|VTF\s+pixel\s+decoding)[^\n.]*\b(?:implemented|supported|validated|certified|passed|passes|works|available|complete|completed)\b", re.I),
        ),
    ),
)


@dataclass(frozen=True)
class AllowEntry:
    entry_id: str
    category: str
    evidence_refs: tuple[str, ...]
    files: tuple[str, ...]
    line_patterns: tuple[re.Pattern[str], ...]

    def matches(self, category: str, rel_path: str, line: str) -> bool:
        if self.category != category:
            return False
        if not self.evidence_refs:
            return False
        if self.files and not any(fnmatch.fnmatch(rel_path, pattern) for pattern in self.files):
            return False
        return any(pattern.search(line) for pattern in self.line_patterns)


@dataclass(frozen=True)
class Finding:
    path: Path
    line_number: int
    category: str
    description: str
    line: str


def load_allowlist(path: Path) -> tuple[AllowEntry, ...]:
    if not path.exists():
        raise SystemExit(f"claim guard allowlist not found: {path}")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid claim guard allowlist JSON {path}: {error}") from error
    entries: list[AllowEntry] = []
    for raw in data.get("entries", []):
        entry_id = str(raw.get("id", "")).strip()
        category = str(raw.get("category", "")).strip()
        evidence_refs = tuple(str(item).strip() for item in raw.get("evidence_refs", []) if str(item).strip())
        files = tuple(str(item).strip() for item in raw.get("files", []) if str(item).strip())
        raw_patterns = [str(item) for item in raw.get("line_patterns", []) if str(item).strip()]
        if not entry_id or not category or not evidence_refs or not raw_patterns:
            raise SystemExit(
                f"allowlist entry {entry_id or '<missing id>'} must include id, category, evidence_refs, and line_patterns"
            )
        try:
            patterns = tuple(re.compile(pattern, re.IGNORECASE) for pattern in raw_patterns)
        except re.error as error:
            raise SystemExit(f"invalid regex in allowlist entry {entry_id}: {error}") from error
        entries.append(AllowEntry(entry_id, category, evidence_refs, files, patterns))
    return tuple(entries)


def iter_scan_paths(root: Path, extras: Sequence[str]) -> list[Path]:
    paths: set[Path] = set()
    for pattern in SCAN_GLOBS:
        for path in root.glob(pattern):
            if path.is_file() and not any(part in SKIP_DIRS for part in path.parts):
                paths.add(path)
    for extra in extras:
        path = (root / extra).resolve() if not Path(extra).is_absolute() else Path(extra).resolve()
        if path.is_file():
            paths.add(path)
        elif path.is_dir():
            for child in path.rglob("*.md"):
                if child.is_file() and not any(part in SKIP_DIRS for part in child.parts):
                    paths.add(child)
        else:
            raise SystemExit(f"extra claim scan path does not exist: {extra}")
    return sorted(paths)


def is_allowed(category: str, rel_path: str, line: str, allowlist: Sequence[AllowEntry]) -> bool:
    if SAFE_CONTEXT.search(line):
        return True
    return any(entry.matches(category, rel_path, line) for entry in allowlist)


def display_path(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()


def scan_file(path: Path, root: Path, allowlist: Sequence[AllowEntry]) -> list[Finding]:
    rel_path = display_path(path, root)
    findings: list[Finding] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), start=1):
        for rule in RULES:
            if any(pattern.search(line) for pattern in rule.patterns):
                if not is_allowed(rule.category, rel_path, line, allowlist):
                    findings.append(Finding(path, line_number, rule.category, rule.description, line.strip()))
    return findings


def scan(root: Path, allowlist_path: Path, extras: Sequence[str]) -> list[Finding]:
    allowlist = load_allowlist(allowlist_path)
    findings: list[Finding] = []
    for path in iter_scan_paths(root, extras):
        findings.extend(scan_file(path, root, allowlist))
    return findings


def print_findings(findings: Sequence[Finding], root: Path) -> None:
    print("validation claim guard failed: unsupported high-risk claims found", file=sys.stderr)
    for finding in findings:
        rel = display_path(finding.path, root)
        print(
            f"{rel}:{finding.line_number}: {finding.category}: {finding.description}: {finding.line}",
            file=sys.stderr,
        )
    print(
        "Update docs/validation-claim-allowlist.json only after recording real evidence rows/issues; "
        "see docs/validation-claim-guard.md.",
        file=sys.stderr,
    )


def self_test() -> None:
    with tempfile.TemporaryDirectory(prefix="sourceweaver-claim-guard-") as tmp:
        root = Path(tmp)
        (root / "docs").mkdir()
        (root / "docs" / "validation-claim-allowlist.json").write_text(
            json.dumps(
                {
                    "version": 1,
                    "entries": [
                        {
                            "id": "test-allowed-signed-update-handoff",
                            "category": "automatic-updates",
                            "evidence_refs": ["test evidence row"],
                            "files": ["CHANGELOG.md"],
                            "line_patterns": ["signed update checks and verified download/install handoff"],
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )
        (root / "README.md").write_text(
            "Hammer/Hammer++ open/save compatibility is not certified.\n"
            "The current map preview is not yet a full textured Hammer clone.\n",
            encoding="utf-8",
        )
        (root / "CHANGELOG.md").write_text(
            "Signed update checks and verified download/install handoff are implemented.\n",
            encoding="utf-8",
        )
        safe_findings = scan(root, root / "docs" / "validation-claim-allowlist.json", [])
        if safe_findings:
            print_findings(safe_findings, root)
            raise SystemExit("self-test expected safe fixture to pass")
        bad = root / "docs" / "bad-release-notes.md"
        bad.write_text(
            "Source Weaver is Hammer++ compatible and native Windows compiler validated.\n"
            "Game runtime map load passed with rendered HLMV preview complete.\n"
            "This is a signed Windows release with automatic updates enabled and a textured Hammer-equivalent preview implemented.\n",
            encoding="utf-8",
        )
        bad_findings = scan(root, root / "docs" / "validation-claim-allowlist.json", [])
        categories = {finding.category for finding in bad_findings}
        expected = {
            "hammer-open-save",
            "native-windows-compiler",
            "game-runtime-load",
            "rendered-hlmv-preview",
            "signed-windows-release",
            "automatic-updates",
            "textured-hammer-equivalent-preview",
        }
        missing = expected - categories
        if missing:
            print_findings(bad_findings, root)
            raise SystemExit(f"self-test missing expected categories: {sorted(missing)}")
    print("validation claim guard self-test passed")


def main(argv: Sequence[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--allowlist", type=Path, default=DEFAULT_ALLOWLIST)
    parser.add_argument("--extra", action="append", default=[], help="additional Markdown file or directory to scan")
    parser.add_argument("--self-test", action="store_true", help="run built-in positive/negative fixtures")
    args = parser.parse_args(argv)

    if args.self_test:
        self_test()
        return 0

    findings = scan(ROOT, args.allowlist, args.extra)
    if findings:
        print_findings(findings, ROOT)
        return 1
    print("validation claim guard passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
