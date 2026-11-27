#!/usr/bin/env python3
"""Rebuild the frozen, fixed-seed benchmark corpus byte-for-byte.

Sources are owner-authored repository documentation only: prose paragraphs plus
code-heavy Markdown lines (paths, API names, identifiers). A fixed seed chooses
the line order, and prose / identifier lines are drawn alternately so the result
is a realistic mixed corpus rather than a synthetic ASCII fixture. The generated
bytes are committed as ``results/corpus.txt``; this script is provenance and is
not invoked by the benchmark (which uses ``include_bytes!``).

Usage: python3 results/make_corpus.py
"""

from pathlib import Path
import random

SEED = 20261126
TARGET_BYTES = 50_000

# Fixed order, fixed relative paths from the repository root.
SOURCES = (
    "projects/bpe-rs/README.md",
    "projects/bpe-rs/WRITEUP.md",
    "docs/corporate-gameplan.md",
    "docs/recommendations.md",
    "docs/activeContext.md",
    "docs/sessions/phase-2/scope-decision.md",
    "docs/sessions/phase-2/scope-expansion.md",
    "docs/sessions/phase-2/ai.md",
    "docs/sessions/phase-2/web.md",
)

# A line is "identifier-like" when it visibly contains code, paths or symbols.
IDENTIFIER_MARKERS = ("`", "::", "_", "()", "=>", ".rs", ".md", "/")


def split_lines(root: Path) -> tuple[list[str], list[str]]:
    prose: list[str] = []
    identifiers: list[str] = []
    for relative in SOURCES:
        text = (root / relative).read_text(encoding="utf-8")
        for raw in text.splitlines():
            line = raw.strip()
            if not line:
                continue
            if any(marker in line for marker in IDENTIFIER_MARKERS):
                identifiers.append(line)
            else:
                prose.append(line)
    return prose, identifiers


def build(root: Path) -> str:
    prose, identifiers = split_lines(root)
    rng = random.Random(SEED)
    rng.shuffle(prose)
    rng.shuffle(identifiers)

    selected: list[str] = []
    total = 0
    turn = 0
    while total < TARGET_BYTES:
        bucket = prose if turn % 2 == 0 else identifiers
        if not bucket:
            bucket = prose if prose else identifiers
        if not bucket:
            break
        line = bucket.pop()
        selected.append(line)
        total += len(line.encode("utf-8")) + 1
        turn += 1
    return "\n".join(selected) + "\n"


def main() -> None:
    # parents: [0]=results, [1]=bpe-rs, [2]=projects, [3]=repository root.
    root = Path(__file__).resolve().parents[3]
    (root / "projects/bpe-rs/results/corpus.txt").write_text(
        build(root), encoding="utf-8"
    )


if __name__ == "__main__":
    main()
