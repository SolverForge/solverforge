// commit-and-tag-version updater for publish-time internal dependency
// requirements in a crate manifest, e.g.
//   solverforge-core = { version = "0.19.6", path = "../solverforge-core" }
// Only internal path dependencies carry a `path = "../solverforge-` segment,
// so external dependency versions are never touched.

const pattern = /(version = ")[^"]*(", path = "\.\.\/solverforge-)/g;

module.exports.readVersion = function (contents) {
  const match = pattern.exec(contents);
  pattern.lastIndex = 0;
  if (!match) {
    throw new Error("internal path dependency version not found");
  }
  return match[0].match(/version = "([^"]*)"/)[1];
};

module.exports.writeVersion = function (contents, version) {
  if (!pattern.test(contents)) {
    pattern.lastIndex = 0;
    throw new Error("internal path dependency version not found");
  }
  pattern.lastIndex = 0;
  return contents.replace(pattern, `$1${version}$2`);
};
