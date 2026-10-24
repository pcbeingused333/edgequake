#!/usr/bin/env python3
"""Validate SPEC-173 pack: relative md links, LAW/F/EC id coverage."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ERRORS: list[str] = []
WARNINGS: list[str] = []

LINK_RE = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")
LAW_RE = re.compile(r"LAW-173-(\d+)")
FINDING_RE = re.compile(r"F-173-(\d+)")
EC_RE = re.compile(r"EC-173-(\d+)")


def resolve_link(src: Path, target: str) -> Path | None:
    if target.startswith(("http://", "https://", "mailto:", "#")):
        return None
    path_part = target.split("#", 1)[0]
    if not path_part:
        return None
    from urllib.parse import unquote

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
            if in_pack:
                ERRORS.append(
                    f"Broken in-pack link in {md.relative_to(ROOT)}: ({target})"
                )
            elif target.startswith("../"):
                WARNINGS.append(
                    f"External/missing link in {md.relative_to(ROOT)}: ({target})"
                )


def check_coverage() -> None:
    laws_doc = (ROOT / "01-first-principles.md").read_text(encoding="utf-8")
    laws = {int(n) for n in LAW_RE.findall(laws_doc)}
    expected_laws = set(range(1, 11))
    if expected_laws - laws:
        ERRORS.append(f"Missing LAW-173 in 01: {sorted(expected_laws - laws)}")

    xref = (ROOT / "09-cross-ref.md").read_text(encoding="utf-8")
    xref_laws = {int(n) for n in LAW_RE.findall(xref)}
    if expected_laws - xref_laws:
        ERRORS.append(
            f"09-cross-ref missing laws: {sorted(expected_laws - xref_laws)}"
        )

    findings_doc = (ROOT / "02-code-facts.md").read_text(encoding="utf-8")
    findings = {int(n) for n in FINDING_RE.findall(findings_doc)}
    expected_findings = set(range(1, 13))
    if expected_findings - findings:
        ERRORS.append(
            f"Missing F-173 in 02-code-facts: {sorted(expected_findings - findings)}"
        )
    xref_findings = {int(n) for n in FINDING_RE.findall(xref)}
    if expected_findings - xref_findings:
        ERRORS.append(
            f"09-cross-ref missing findings: {sorted(expected_findings - xref_findings)}"
        )

    ec_doc = (ROOT / "06-edge-cases.md").read_text(encoding="utf-8")
    ecs = {int(n) for n in EC_RE.findall(ec_doc)}
    expected_ecs = set(range(1, 31))
    if expected_ecs - ecs:
        ERRORS.append(f"Missing EC-173 in 06: {sorted(expected_ecs - ecs)}")

    matrix = (ROOT / "08-e2e-test-matrix.md").read_text(encoding="utf-8")
    missing_in_matrix: list[int] = []
    for n in sorted(ecs):
        padded = f"{n:02d}"
        if (
            f"EC-173-{padded}" in matrix
            or f"EC-173-{n}" in matrix
            or f"T-173-{padded}" in matrix
        ):
            continue
        missing_in_matrix.append(n)
    if missing_in_matrix:
        ERRORS.append(
            f"ECs in 06 missing from 08 matrix coverage: {missing_in_matrix}"
        )

    xref_ecs = {int(n) for n in EC_RE.findall(xref)}
    if expected_ecs - xref_ecs:
        ERRORS.append(
            f"09-cross-ref missing edge cases: {sorted(expected_ecs - xref_ecs)}"
        )

    required_files = [
        "README.md",
        "00-why.md",
        "01-first-principles.md",
        "02-code-facts.md",
        "03-protocol.md",
        "04-architecture.md",
        "05-contract-delta.md",
        "06-edge-cases.md",
        "07-implementation-plan.md",
        "08-e2e-test-matrix.md",
        "09-cross-ref.md",
        "lenses/LENS-product-owner.md",
        "lenses/LENS-full-stack.md",
        "lenses/LENS-database.md",
        "lenses/LENS-ux-ui.md",
        "lenses/LENS-front.md",
        "lenses/LENS-ai-engineer.md",
        "lenses/LENS-mcp.md",
        "lenses/LENS-security.md",
        "lenses/LENS-object-store.md",
    ]
    for rel in required_files:
        if not (ROOT / rel).exists():
            ERRORS.append(f"Missing required file: {rel}")

    for rel in required_files:
        if not rel.startswith("lenses/"):
            continue
        text = (ROOT / rel).read_text(encoding="utf-8")
        if "LAW-173-" not in text or "EC-173-" not in text:
            ERRORS.append(f"{rel} must cite at least one LAW-173 and one EC-173")


def main() -> int:
    check_links()
    check_coverage()
    for w in WARNINGS:
        print(f"WARN: {w}")
    for e in ERRORS:
        print(f"ERROR: {e}")
    if ERRORS:
        print(f"FAILED: {len(ERRORS)} error(s), {len(WARNINGS)} warning(s)")
        return 1
    print(f"OK: SPEC-173 pack valid ({len(WARNINGS)} warning(s)). Root={ROOT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
