#!/usr/bin/env python3
"""Validate the SPEC-172 pack: links, laws, findings, WHYs, edge cases."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []
WARNINGS: list[str] = []

LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
LAW_RE = re.compile(r"LAW-172-(\d+)")
FINDING_RE = re.compile(r"F-172-(\d+)")
EC_RE = re.compile(r"EC-172-(\d+)")
WHY_RE = re.compile(r"WHY-172-(\d+)")

REQUIRED = [
    "README.md",
    "00-why.md",
    "01-first-principles.md",
    "02-code-facts.md",
    "03-object-store-theory.md",
    "04-architecture.md",
    "05-edge-cases.md",
    "06-implementation-plan.md",
    "07-e2e-test-matrix.md",
    "08-migration-ux.md",
    "09-cross-ref.md",
    "lenses/LENS-product-owner.md",
    "lenses/LENS-full-stack.md",
    "lenses/LENS-database.md",
    "lenses/LENS-ux.md",
    "lenses/LENS-front.md",
    "lenses/LENS-object-store.md",
    "lenses/LENS-security.md",
    "lenses/LENS-system-design.md",
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
    laws = set(range(1, 9))
    findings = set(range(1, 22))
    whys = set(range(1, 6))
    ecs = set(range(1, 41))

    expect_ids(ROOT / "01-first-principles.md", LAW_RE, laws, "LAW-172")
    expect_ids(ROOT / "09-cross-ref.md", LAW_RE, laws, "LAW-172")
    expect_ids(ROOT / "02-code-facts.md", FINDING_RE, findings, "F-172")
    expect_ids(ROOT / "09-cross-ref.md", FINDING_RE, findings, "F-172")
    expect_ids(ROOT / "00-why.md", WHY_RE, whys, "WHY-172")
    expect_ids(ROOT / "09-cross-ref.md", WHY_RE, whys, "WHY-172")
    expect_ids(ROOT / "05-edge-cases.md", EC_RE, ecs, "EC-172")

    matrix = (ROOT / "07-e2e-test-matrix.md").read_text(encoding="utf-8")
    missing_ecs = sorted(n for n in ecs if f"EC-172-{n:02d}" not in matrix)
    if missing_ecs:
        ERRORS.append(f"ECs missing from 07 matrix: {missing_ecs}")

    plan = (ROOT / "06-implementation-plan.md").read_text(encoding="utf-8")
    cross = (ROOT / "09-cross-ref.md").read_text(encoding="utf-8")
    for wave in ("W0", "W1", "W2", "W3", "W4", "W5", "W6"):
        if wave not in plan:
            ERRORS.append(f"06 missing {wave}")
        if wave not in cross:
            ERRORS.append(f"09 missing {wave}")


def check_required() -> None:
    for rel in REQUIRED:
        if not (ROOT / rel).is_file():
            ERRORS.append(f"Missing file {rel}")


def check_diagrams() -> None:
    for name in ("00-why.md", "03-object-store-theory.md", "04-architecture.md"):
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
    print(f"SPEC-172 cross-ref ok ({len(list(ROOT.rglob('*.md')))} markdown files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
