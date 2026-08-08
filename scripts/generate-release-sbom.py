#!/usr/bin/env python3
"""Generate a lightweight CycloneDX SBOM for the Source Weaver Rust workspace."""
from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
import sys
import uuid
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]


def run_cargo_metadata(root: Path) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=root,
        check=True,
        text=True,
        capture_output=True,
    )
    return json.loads(result.stdout)


def package_ref(package: dict[str, Any]) -> str:
    return f"pkg:cargo/{package['name']}@{package['version']}"


def component_type(package: dict[str, Any], workspace_ids: set[str]) -> str:
    return "application" if package["id"] in workspace_ids else "library"


def license_entries(package: dict[str, Any]) -> list[dict[str, str]] | None:
    license_value = package.get("license")
    if license_value:
        return [{"expression": license_value}]
    return None


def external_refs(package: dict[str, Any]) -> list[dict[str, str]] | None:
    refs: list[dict[str, str]] = []
    repository = package.get("repository")
    homepage = package.get("homepage")
    documentation = package.get("documentation")
    if repository:
        refs.append({"type": "vcs", "url": repository})
    if homepage:
        refs.append({"type": "website", "url": homepage})
    if documentation:
        refs.append({"type": "documentation", "url": documentation})
    return refs or None


def build_component(package: dict[str, Any], workspace_ids: set[str]) -> dict[str, Any]:
    component: dict[str, Any] = {
        "type": component_type(package, workspace_ids),
        "bom-ref": package_ref(package),
        "name": package["name"],
        "version": package["version"],
        "purl": package_ref(package),
    }
    if package.get("description"):
        component["description"] = package["description"]
    licenses = license_entries(package)
    if licenses:
        component["licenses"] = licenses
    refs = external_refs(package)
    if refs:
        component["externalReferences"] = refs
    return component


def dependency_refs(metadata: dict[str, Any]) -> list[dict[str, Any]]:
    packages_by_id = {package["id"]: package for package in metadata["packages"]}
    dependencies: list[dict[str, Any]] = []
    for node in metadata.get("resolve", {}).get("nodes", []):
        package = packages_by_id.get(node["id"])
        if not package:
            continue
        refs: list[str] = []
        for dependency in node.get("deps", []):
            dep_package = packages_by_id.get(dependency["pkg"])
            if dep_package:
                refs.append(package_ref(dep_package))
        dependencies.append({"ref": package_ref(package), "dependsOn": sorted(set(refs))})
    return sorted(dependencies, key=lambda item: item["ref"])


def build_sbom(metadata: dict[str, Any], root: Path) -> dict[str, Any]:
    workspace_ids = set(metadata.get("workspace_members", []))
    packages = sorted(metadata["packages"], key=lambda package: (package["name"], package["version"], package["id"]))
    root_package = next((package for package in packages if package["name"] == "sourceweaver-cli"), packages[0])
    now = dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    return {
        "$schema": "https://cyclonedx.org/schema/bom-1.5.schema.json",
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": f"urn:uuid:{uuid.uuid4()}",
        "version": 1,
        "metadata": {
            "timestamp": now,
            "tools": {
                "components": [
                    {
                        "type": "application",
                        "name": "sourceweaver-release-sbom-generator",
                        "version": "1",
                        "description": "Repository-local SBOM generator using cargo metadata --locked.",
                    }
                ]
            },
            "component": {
                "type": "application",
                "bom-ref": "pkg:github/dayflaree/sourceweaver",
                "name": "Source Weaver",
                "version": root_package.get("version", "0.0.0"),
                "description": "Source Weaver release artifact Rust workspace SBOM.",
                "externalReferences": [
                    {"type": "vcs", "url": "https://github.com/dayflaree/sourceweaver"}
                ],
            },
            "properties": [
                {"name": "sourceweaver:sbom-source", "value": "cargo metadata --locked --format-version 1"},
                {"name": "sourceweaver:repository", "value": root.as_posix()},
            ],
        },
        "components": [build_component(package, workspace_ids) for package in packages],
        "dependencies": dependency_refs(metadata),
    }


def validate_sbom(sbom: dict[str, Any]) -> None:
    required = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
    }
    for key, value in required.items():
        if sbom.get(key) != value:
            raise ValueError(f"expected {key}={value!r}, found {sbom.get(key)!r}")
    components = sbom.get("components")
    if not isinstance(components, list) or not components:
        raise ValueError("SBOM contains no components")
    names = {component.get("name") for component in components}
    for expected in {"sourceweaver-cli", "sourceweaver-core", "sourceweaver-desktop"}:
        if expected not in names:
            raise ValueError(f"workspace component missing from SBOM: {expected}")
    if not isinstance(sbom.get("dependencies"), list) or not sbom["dependencies"]:
        raise ValueError("SBOM contains no dependency graph")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("target/release-artifacts/sourceweaver-sbom.cdx.json"))
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--validate-only", type=Path, help="validate an existing SBOM JSON file")
    args = parser.parse_args(argv)

    if args.validate_only:
        sbom = json.loads(args.validate_only.read_text(encoding="utf-8"))
        validate_sbom(sbom)
        print(f"validated {args.validate_only}")
        return 0

    metadata = run_cargo_metadata(args.root)
    sbom = build_sbom(metadata, args.root)
    validate_sbom(sbom)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(sbom, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
