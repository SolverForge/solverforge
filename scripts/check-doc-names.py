#!/usr/bin/env python3
"""Fail when a wireframe or guide lists a Rust type, trait or enum that no
longer exists in the workspace sources.

Checks every bullet that starts with a backticked CamelCase name
(`- `Name``, `- `Name<T>``, `- `A`, `B``) and every `### `Name`` /
`#### `Name`` heading in crates/*/WIREFRAME.md and docs/*.md. A name counts as
existing when some crates/*/src file defines it (struct, enum, trait, type,
union, enum variant), imports or re-exports it in a `use` item, or it is a
standard-library trait. Run from the repository root; prints the missing names
with their locations and exits 1 if any.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
# CamelCase: an upper-case letter followed by a lower-case one, so constants
# such as `ZERO` are not taken for types.
NAME = r"[A-Z][a-z][A-Za-z0-9]*"
STD_TRAITS = {
    "Add", "Clone", "Copy", "Debug", "Default", "Deserialize", "Display", "Drop",
    "Eq", "Hash", "Iterator", "Neg", "Ord", "PartialEq", "PartialOrd", "Send",
    "Serialize", "Sub", "Sync",
}
BULLET = re.compile(rf"^\s*- `({NAME})(?:[<`( ])")
HEADING = re.compile(rf"^#{{3,4}} `({NAME})(?:[<` ])")
SIBLING = re.compile(rf"`({NAME})(?:<[^`]*>)?`")


def defined_names():
    item = re.compile(rf"\b(?:struct|enum|trait|type|union)\s+({NAME})\b")
    variant = re.compile(rf"^\s+({NAME})\s*[({{,]", re.MULTILINE)
    use = re.compile(r"\buse\s[^;]*;")
    names = set(STD_TRAITS)
    for path in ROOT.glob("crates/*/src/**/*.rs"):
        text = path.read_text(encoding="utf-8")
        names.update(item.findall(text))
        names.update(variant.findall(text))
        for statement in use.findall(text):
            names.update(re.findall(NAME, statement))
    return names


def listed_names(path):
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        bullet = BULLET.match(line)
        if bullet:
            yield number, bullet.group(1)
            # `- `A`, `B`, `C`` lists several exports on one bullet.
            head = line.split(" — ")[0].split(" (")[0]
            if re.fullmatch(rf"\s*- (?:`{NAME}(?:<[^`]*>)?`(?:, |$))+", head):
                for sibling in SIBLING.findall(head)[1:]:
                    yield number, sibling
            continue
        heading = HEADING.match(line)
        if heading:
            yield number, heading.group(1)


def main():
    defined = defined_names()
    documents = sorted(ROOT.glob("crates/*/WIREFRAME.md")) + sorted(ROOT.glob("docs/*.md"))
    missing = [
        f"{path.relative_to(ROOT)}:{number}: `{name}`"
        for path in documents
        for number, name in listed_names(path)
        if name not in defined
    ]
    if missing:
        print("Documented names with no definition in crates/*/src:")
        print("\n".join(missing))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
