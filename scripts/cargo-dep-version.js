// Version updater for versioned path dependencies in Cargo.toml
// Updates lines like: solverforge-core = { version = "0.2.1", path = "..." }

module.exports.readVersion = function (contents) {
  const match = contents.match(/solverforge-\w+\s*=\s*\{\s*version\s*=\s*"([^"]+)"/);
  return match ? match[1] : null;
};

module.exports.writeVersion = function (contents, version) {
  // Update all solverforge-* versioned path dependencies
  return contents.replace(
    /(solverforge-\w+\s*=\s*\{\s*version\s*=\s*")[^"]+(")/g,
    `$1${version}$2`
  );
};
