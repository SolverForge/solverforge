// commit-and-tag-version updater for the AGENTS.md release version line.

const pattern = /(Current workspace release version: `)[^`]*(`)/;

module.exports.readVersion = function (contents) {
  const match = contents.match(pattern);
  if (!match) {
    throw new Error("workspace release version not found in AGENTS.md");
  }
  return match[0].match(/`([^`]*)`/)[1];
};

module.exports.writeVersion = function (contents, version) {
  if (!pattern.test(contents)) {
    throw new Error("workspace release version not found in AGENTS.md");
  }
  return contents.replace(pattern, `$1${version}$2`);
};
