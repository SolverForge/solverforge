// commit-and-tag-version updater for the workspace release version.
// The workspace version is the single source of truth: every crate manifest
// inherits it through `version.workspace = true`.

const pattern = /^(\[workspace\.package\][\s\S]*?^version = ")[^"]*(")/m;

module.exports.readVersion = function (contents) {
  const match = contents.match(pattern);
  if (!match) {
    throw new Error("workspace version not found in Cargo.toml");
  }
  return match[0].match(/version = "([^"]*)"/)[1];
};

module.exports.writeVersion = function (contents, version) {
  if (!pattern.test(contents)) {
    throw new Error("workspace version not found in Cargo.toml");
  }
  return contents.replace(pattern, `$1${version}$2`);
};
