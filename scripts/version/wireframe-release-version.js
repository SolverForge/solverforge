// commit-and-tag-version updater for a crate wireframe release line,
// e.g. `**Workspace Release:** \`0.19.6\``.

const pattern = /(\*\*Workspace Release:\*\* `)[^`]*(`)/;

module.exports.readVersion = function (contents) {
  const match = contents.match(pattern);
  if (!match) {
    throw new Error("workspace release version not found in wireframe");
  }
  return match[0].match(/`([^`]*)`/)[1];
};

module.exports.writeVersion = function (contents, version) {
  if (!pattern.test(contents)) {
    throw new Error("workspace release version not found in wireframe");
  }
  return contents.replace(pattern, `$1${version}$2`);
};
