#!/usr/bin/env python3
"""Validate the SPEC-171 pack: links, laws, findings, WHYs, edge cases."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []
WARNINGS: list[str] = []

LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
LAW_RE = re.compile(r"LAW-171-(\d+)")
FINDING_RE = re.compile(r"F-171-(\d+)")
EC_RE = re.compile(r"EC-171-(\d+)")
WHY_RE = re.compile(r"WHY-171-(\d+)")

REQUIRED = [
    "README.md",
    "00-why.md",
    "01-first-principles.md",
    "02-incident-catalogue.md",
    "03-code-facts.md",
    "04-postgres-theory.md",
    "05-architecture.md",
    "06-edge-cases.md",
    "07-implementation-plan.md",
    "08-e2e-test-matrix.md",
    "09-cross-ref.md",
    "lenses/LENS-product-owner.md",
    "lenses/LENS-full-stack.md",
    "lenses/LENS-database.md",
    "lenses/LENS-system-design.md",
    "lenses/LENS-postgres.md",
    "lenses/LENS-reliability.md",
]


def resolve_link(src: Path, target: str) -> Path | None:
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return None
    path_part = target.split("#", 1)[0]
    if not path_part:
        return None
    return (src.parent / unquote(path_part)).resolve()


def check_links() -> None:
    for md in ROOT.rglob("*.md"):
        if "00-raw" in md.parts:
            continue
        text = md.read_text(encoding="utf-8")
        for _label, target in LINK_RE.findall(text):
            dest = resolve_link(md, target)
            if dest is None:
                continue
            if dest.exists():
                continue
            try:
                dest.relative_to(ROOT)
                in_pack = True
            except ValueError:
                in_pack = False
            rel = md.relative_to(ROOT)
            if in_pack:
                ERRORS.append(f"Broken in-pack link in {rel}: ({target})")
            elif target.startswith("../"):
                WARNINGS.append(f"External/missing link in {rel}: ({target})")


def expect_ids(path: Path, pattern: re.Pattern[str], expected: set[int], label: str) -> None:
    found = {int(n) for n in pattern.findall(path.read_text(encoding="utf-8"))}
    missing = expected - found
    if missing:
        ERRORS.append(f"Missing {label} in {path.name}: {sorted(missing)}")


def check_coverage() -> None:
    laws = set(range(1, 7))
    findings = set(range(1, 19))
    whys = set(range(1, 6))
    ecs = set(range(1, 41))

    expect_ids(ROOT / "01-first-principles.md", LAW_RE, laws, "LAW-171")
    expect_ids(ROOT / "09-cross-ref.md", LAW_RE, laws, "LAW-171")
    expect_ids(ROOT / "03-code-facts.md", FINDING_RE, findings, "F-171")
    expect_ids(ROOT / "09-cross-ref.md", FINDING_RE, findings, "F-171")
    expect_ids(ROOT / "00-why.md", WHY_RE, whys, "WHY-171")
    expect_ids(ROOT / "09-cross-ref.md", WHY_RE, whys, "WHY-171")
    expect_ids(ROOT / "06-edge-cases.md", EC_RE, ecs, "EC-171")

    matrix = (ROOT / "08-e2e-test-matrix.md").read_text(encoding="utf-8")
    missing_ecs = sorted(n for n in ecs if f"EC-171-{n:02d}" not in matrix)
    if missing_ecs:
        ERRORS.append(f"ECs missing from 08 matrix: {missing_ecs}")

    plan = (ROOT / "07-implementation-plan.md").read_text(encoding="utf-8")
    for wave in ("W0", "W1", "W2", "W3", "W4", "W5", "W6"):
        if wave not in plan:
            ERRORS.append(f"07 missing {wave}")
        if wave not in (ROOT / "09-cross-ref.md").read_text(encoding="utf-8"):
            ERRORS.append(f"09 missing {wave}")


def check_required() -> None:
    for rel in REQUIRED:
        if not (ROOT / rel).is_file():
            ERRORS.append(f"Missing file {rel}")


def check_diagrams() -> None:
    for name in ("00-why.md", "04-postgres-theory.md", "05-architecture.md"):
        text = (ROOT / name).read_text(encoding="utf-8")
        if "```mermaid" not in text:
            ERRORS.append(f"{name} has no mermaid diagram")
        if "┌" not in text:
            ERRORS.append(f"{name} has no ASCII box diagram")
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    if "┌" not in readme:
        ERRORS.append("README.md has no ASCII box diagram")


def main() -> int:
    check_required()
    check_links()
    check_coverage()
    check_diagrams()
    for warning in WARNINGS:
        print(f"warn: {warning}")
    if ERRORS:
        for error in ERRORS:
            print(f"error: {error}")
        return 1
    print(
        f"SPEC-171 cross-ref ok ({len(list(ROOT.rglob('*.md')))} markdown files)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
