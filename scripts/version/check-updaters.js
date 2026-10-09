// Verify all configured release surfaces without modifying the working tree.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const config = JSON.parse(fs.readFileSync(path.join(root, ".versionrc.json")));
const primary = config.packageFiles[0];
const current = require(path.join(root, primary.updater)).readVersion(
  fs.readFileSync(path.join(root, primary.filename), "utf8"),
);
const next = "99.98.97";

for (const entry of config.bumpFiles) {
  const updater = require(path.join(root, entry.updater));
  const contents = fs.readFileSync(path.join(root, entry.filename), "utf8");
  assert.equal(updater.readVersion(contents), current, entry.filename);
  const bumped = updater.writeVersion(contents, next);
  assert.equal(updater.readVersion(bumped), next, entry.filename);
  assert.equal(updater.writeVersion(bumped, current), contents, entry.filename);
  assert(!bumped.includes("$3"), entry.filename);
}

const readme = require("./readme-updater.js");
const fixture = `**Current workspace version:** 1.2.3\nsolverforge = { version = "1.2.3", features = ["console"] }\nv1.2.3 - Zero-Erasure Constraint Solver\n`;
const expected = fixture.replaceAll("1.2.3", next);
assert.equal(readme.writeVersion(fixture, next), expected);
assert.equal(readme.writeVersion(expected, next), expected);
console.log("Release updater round-trips and README regression checks passed");
