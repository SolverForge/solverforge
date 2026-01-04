// Version updater for Cargo.toml
const fs = require('fs');

module.exports.readVersion = function (contents) {
  const match = contents.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    const simpleMatch = contents.match(/^version\s*=\s*"([^"]+)"/m);
    return simpleMatch ? simpleMatch[1] : null;
  }
  return match[1];
};

module.exports.writeVersion = function (contents, version) {
  // Update version in [workspace.package] section
  return contents.replace(
    /^(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
    `$1${version}$2`
  );
};
