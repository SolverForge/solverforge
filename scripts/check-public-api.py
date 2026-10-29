#!/usr/bin/env python3
"""Fail when the public API of core, scoring or solver differs from its snapshot.

Reads the rustdoc HTML of each crate (run
`cargo doc -p solverforge-core -p solverforge-scoring -p solverforge-solver
--no-deps --all-features` first) and writes one line per public item: its
declaration, then the signature of each inherent method or trait member.
Trait implementations are left out, because they are listed on the trait's own
page. `#[doc(hidden)]` items have no pages and are not part of the snapshot.

The snapshot of each crate lives in `crates/<crate>/public-api.txt`. A change
to the public surface must update that file in the same commit, so review sees
every item that is added, removed or changed. `--bless` rewrites the snapshots.
"""

import html
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
CRATES = ["solverforge-core", "solverforge-scoring", "solverforge-solver"]
ITEM = re.compile(r'<li><a href="([^"]+)">([^<]+)</a></li>')
DECLARATION = re.compile(r'<pre class="rust item-decl">(.*?)</pre>', re.S)
MEMBER = re.compile(r'<h4 class="code-header">(.*?)</h4>', re.S)
# Where the parts of a page that describe other types begin.
OTHER_TYPES = re.compile(
    r'<h2 id="(?:trait-implementations|synthetic-implementations|'
    r'blanket-implementations|foreign-impls|implementors|object-safety|dyn-compatibility)"'
)
# Rustdoc's notable-trait marker and the line break before a where clause.
NOTABLE = re.compile(r'<a [^>]*class="tooltip"[^>]*>.*?</a>', re.S)
WHERE = re.compile(r'<(?:div|span) class="where[^"]*">')
TAG = re.compile(r"<[^>]+>")
SPACE = re.compile(r"\s+")


def plain(fragment: str) -> str:
    fragment = WHERE.sub(" ", NOTABLE.sub("", fragment))
    text = html.unescape(TAG.sub("", fragment))
    return SPACE.sub(" ", text).strip()


def snapshot(crate: str) -> list[str]:
    docs = ROOT / "target" / "doc" / crate.replace("-", "_")
    index = (docs / "all.html").read_text()
    lines = []
    for href, path in ITEM.findall(index):
        page = (docs / href).read_text(errors="ignore")
        kind = href.split("/")[-1].split(".")[0]
        other = OTHER_TYPES.search(page)
        if other:
            page = page[: other.start()]
        declaration = DECLARATION.search(page)
        lines.append(f"{kind} {path}: {plain(declaration.group(1)) if declaration else ''}")
        for member in MEMBER.findall(page):
            lines.append(f"    {path}: {plain(member)}")
    return lines


def main() -> int:
    bless = "--bless" in sys.argv[1:]
    stale = []
    for crate in CRATES:
        if not (ROOT / "target" / "doc" / crate.replace("-", "_") / "all.html").is_file():
            print(f"rustdoc for {crate} is missing; run `cargo doc -p {crate} --no-deps --all-features`")
            return 1
        current = "\n".join(snapshot(crate)) + "\n"
        file = ROOT / "crates" / crate / "public-api.txt"
        if bless:
            file.write_text(current)
            continue
        recorded = file.read_text() if file.is_file() else ""
        if recorded != current:
            old, new = set(recorded.splitlines()), set(current.splitlines())
            stale.append(crate)
            print(f"{crate}: public API differs from {file.relative_to(ROOT)}")
            for line in sorted(old - new):
                print(f"  - {line.strip()}")
            for line in sorted(new - old):
                print(f"  + {line.strip()}")
    if stale:
        print("Update the snapshots with `python3 scripts/check-public-api.py --bless`.")
    return 1 if stale else 0


if __name__ == "__main__":
    sys.exit(main())
