// Version updater for pom.xml
module.exports.readVersion = function (contents) {
  // Match the first <version> tag (project version, not dependency versions)
  const match = contents.match(/<project[^>]*>[\s\S]*?<version>([^<]+)<\/version>/);
  return match ? match[1] : null;
};

module.exports.writeVersion = function (contents, version) {
  // Replace only the first <version> tag (project version)
  let replaced = false;
  return contents.replace(/<version>([^<]+)<\/version>/, (match, oldVersion) => {
    if (!replaced) {
      replaced = true;
      return `<version>${version}</version>`;
    }
    return match;
  });
};
