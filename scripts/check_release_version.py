"""Check that a release tag matches the project and every packaged plugin version."""
from __future__ import annotations

import json
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
TAG = re.compile(r"v([0-9]+\.[0-9]+\.[0-9]+)(?:-([0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*))?\Z")


def validate(tag: str, project_version: str, plugins: dict[str, str]) -> None:
    match = TAG.fullmatch(tag)
    if not match:
        raise ValueError("Expected v<major>.<minor>.<patch> with an optional prerelease suffix")
    version = match.group(1)
    if version != project_version:
        raise ValueError(f"Tag version {version} does not match Cargo.toml version {project_version}")
    if not plugins:
        raise ValueError("No plugin manifests found")
    mismatched = sorted(name for name, plugin_version in plugins.items() if plugin_version != version)
    if mismatched:
        raise ValueError(f"Plugin versions do not match {version}: {', '.join(mismatched)}")


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python scripts/check_release_version.py v<version>")
    project = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    versions = {}
    for path in sorted((ROOT / "plugins").glob("*/plugin.json")):
        manifest = json.loads(path.read_text(encoding="utf-8"))
        versions[path.parent.name] = manifest.get("version", "")
    try:
        validate(sys.argv[1], project["package"]["version"], versions)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    print(f"Release {sys.argv[1]} matches {len(versions)} plugin manifests")


if __name__ == "__main__":
    main()
