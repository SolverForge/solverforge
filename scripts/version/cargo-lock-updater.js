// commit-and-tag-version updater for workspace member versions in Cargo.lock.
// Every workspace crate shares the workspace package version, and each lock
// package block lists `name = "solverforge-*"` immediately before its version.

const pattern =
  /(\[\[package\]\]\nname = "solverforge[^"]*"\nversion = ")[^"]*(")/g;

module.exports.readVersion = function (contents) {
  const match = pattern.exec(contents);
  pattern.lastIndex = 0;
  if (!match) {
    throw new Error("workspace member version not found in Cargo.lock");
  }
  return match[0].match(/version = "([^"]*)"/)[1];
};

module.exports.writeVersion = function (contents, version) {
  if (!pattern.test(contents)) {
    pattern.lastIndex = 0;
    throw new Error("workspace member version not found in Cargo.lock");
  }
  pattern.lastIndex = 0;
  return contents.replace(pattern, `$1${version}$2`);
};
