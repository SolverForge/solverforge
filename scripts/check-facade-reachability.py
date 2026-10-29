#!/usr/bin/env python3
"""Fail when a facade signature uses a type the facade does not export.

Reads the rustdoc HTML of the `solverforge` crate (run
`cargo doc -p solverforge --no-deps --all-features` first). Rustdoc inlines
everything the facade re-exports, so a link from a facade item declaration or
method signature to another crate's pages names a type that a project
depending only on `solverforge` cannot import. Crates the facade re-exports
whole (`pub use solverforge_bridge as bridge`) count as reachable. A trait's
implementor and foreign-type listings describe other types, not the trait's
signature, so they are not scanned; hidden `__internal` pages are skipped. Prints each such type with the facade pages that use it and exits 1
if there are any.
"""

import collections
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
DOCS = ROOT / "target" / "doc" / "solverforge"
SIGNATURE = re.compile(
    r'<pre class="rust item-decl">(.*?)</pre>|<h4 class="code-header">(.*?)</h4>', re.S
)
FOREIGN = re.compile(r'href="(?:\.\./)+(solverforge_[a-z_]+)/([^"#]+)\.html"')
# Where a trait page's listings of implementing types begin.
LISTINGS = re.compile(r'<h2 id="(?:foreign-impls|implementors)"')
CRATE_ALIAS = re.compile(r"^pub use (solverforge_[a-z_]+) as \w+;", re.M)


def main() -> int:
    if not DOCS.is_dir():
        print(f"{DOCS} is missing; run `cargo doc -p solverforge --no-deps --all-features`")
        return 1
    aliased = set(CRATE_ALIAS.findall((ROOT / "crates/solverforge/src/lib.rs").read_text()))
    unreachable = collections.defaultdict(set)
    for page in DOCS.rglob("*.html"):
        relative = page.relative_to(DOCS)
        if relative.parts[0] == "__internal":
            continue
        text = page.read_text(errors="ignore")
        listings = LISTINGS.search(text)
        if listings:
            text = text[: listings.start()]
        for declaration, method in SIGNATURE.findall(text):
            for crate, path in FOREIGN.findall(declaration or method):
                if crate not in aliased:
                    unreachable[f"{crate}::{path.replace('/', '::')}"].add(str(relative))
    for name in sorted(unreachable):
        pages = sorted(unreachable[name])
        print(f"{name}: used by {', '.join(pages[:3])}{' …' if len(pages) > 3 else ''}")
    return 1 if unreachable else 0


if __name__ == "__main__":
    sys.exit(main())
