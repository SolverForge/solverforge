// commit-and-tag-version updater for the centrally documented README version.

const patterns = [/Current workspace version:\*\* ([0-9]+\.[0-9]+\.[0-9]+)/];

module.exports.readVersion = function (contents) {
  for (const pattern of patterns) {
    const match = contents.match(pattern);
    if (match) {
      return match[1];
    }
  }
  throw new Error("workspace version not found in README.md");
};

module.exports.writeVersion = function (contents, version) {
  let updated = contents;
  let replaced = false;
  for (const pattern of patterns) {
    if (pattern.test(updated)) {
      updated = updated.replace(pattern, `Current workspace version:** ${version}`);
      replaced = true;
    }
  }
  if (!replaced) {
    throw new Error("workspace version not found in README.md");
  }
  return updated;
};
