// commit-and-tag-version updater for the centrally documented README versions.
// The status line is authoritative for reads; the install snippet and console
// banner are also release surfaces that must move together.

const VERSION = String.raw`[0-9]+\.[0-9]+\.[0-9]+`;
const patterns = [
  new RegExp(String.raw`(Current workspace version:\*\* )(${VERSION})`, "g"),
  new RegExp(
    String.raw`(solverforge = \{ version = ")(${VERSION})(", features = \["console"\])`,
    "g",
  ),
  new RegExp(String.raw`(\bv)(${VERSION})( - Zero-Erasure Constraint Solver)`, "g"),
];

module.exports.readVersion = function (contents) {
  for (const pattern of patterns) {
    pattern.lastIndex = 0;
    const match = pattern.exec(contents);
    if (match) {
      return match[2];
    }
  }
  throw new Error("workspace version not found in README.md");
};

module.exports.writeVersion = function (contents, version) {
  let updated = contents;
  let replaced = false;
  for (const pattern of patterns) {
    pattern.lastIndex = 0;
    if (pattern.test(updated)) {
      pattern.lastIndex = 0;
      updated = updated.replace(pattern, `$1${version}$3`);
      replaced = true;
    }
  }
  if (!replaced) {
    throw new Error("workspace version not found in README.md");
  }
  return updated;
};
