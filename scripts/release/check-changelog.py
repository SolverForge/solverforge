#!/usr/bin/env python3
"""Fail when the CHANGELOG section csemver is about to add would damage the
release history.

Runs `csemver --dry-run` (which also proves every configured version surface
resolves) and reads the `## [x.y.z](compare-url)` heading it would write. The
check fails when:

- CHANGELOG.md already has a heading for that version, so a retried release
  would add a second section;
- the compare link is not an absolute https://github.com/SolverForge/solverforge
  URL;
- the version the link compares from has no `v` tag, so the link would 404,
  or is not older than the new version (csemver compares from the newest tag,
  so an existing newer tag would make it release backwards);
- the `v` tag for the new version already exists locally or on the release
  remote, so the release would collide with an earlier attempt.

Run from the repository root. `RELEASE_REMOTE` names the GitHub remote to read
tags from (default: the remote whose URL points at SolverForge/solverforge).
"""

import os
import re
import subprocess
import sys

REPO_URL = "https://github.com/SolverForge/solverforge"
HEADING = re.compile(r"^## \[(?P<version>[0-9]+\.[0-9]+\.[0-9]+)\]\((?P<url>[^)]*)\)")


def run(*args):
    return subprocess.run(args, capture_output=True, text=True)


def release_remote():
    configured = os.environ.get("RELEASE_REMOTE")
    if configured:
        return configured
    for line in run("git", "remote", "-v").stdout.splitlines():
        name, url = line.split()[:2]
        if "SolverForge/solverforge" in url:
            return name
    return None


def remote_tags(remote):
    if remote is None:
        return set()
    listing = run("timeout", "30", "git", "ls-remote", "--tags", remote)
    if listing.returncode != 0:
        sys.exit(f"cannot read tags from `{remote}`:\n{listing.stderr}")
    return {
        line.split("refs/tags/", 1)[1].removesuffix("^{}")
        for line in listing.stdout.splitlines()
        if "refs/tags/" in line
    }


def semver(version):
    return tuple(int(part) for part in version.split("."))


def local_tag_exists(tag):
    return run("git", "rev-parse", "-q", "--verify", f"refs/tags/{tag}").returncode == 0


def main():
    dry_run = run("csemver", "--dry-run")
    if dry_run.returncode != 0:
        sys.exit(f"csemver --dry-run failed:\n{dry_run.stdout}{dry_run.stderr}")
    headings = [m for line in dry_run.stdout.splitlines() if (m := HEADING.match(line))]
    if len(headings) != 1:
        sys.exit(f"expected one new CHANGELOG heading in the dry run, found {len(headings)}")
    version, url = headings[0]["version"], headings[0]["url"]
    tag = f"v{version}"

    errors = []
    with open("CHANGELOG.md", encoding="utf-8") as changelog:
        for number, line in enumerate(changelog, 1):
            if re.match(rf"^## \[?{re.escape(version)}\b", line):
                errors.append(f"CHANGELOG.md:{number} already has a section for {version}")

    compare = re.fullmatch(
        rf"{re.escape(REPO_URL)}/compare/(?P<previous>v[0-9]+\.[0-9]+\.[0-9]+)\.\.\.{re.escape(tag)}",
        url,
    )
    if compare is None:
        errors.append(f"compare link `{url}` is not {REPO_URL}/compare/v<previous>...{tag}")

    remote = release_remote()
    tags_on_remote = remote_tags(remote)
    if compare is not None:
        previous = compare["previous"]
        if not local_tag_exists(previous) and previous not in tags_on_remote:
            errors.append(f"compare link starts at `{previous}`, which is not a tag")
        if semver(previous[1:]) >= semver(version):
            errors.append(f"{tag} does not come after the previous tag `{previous}`")
    if local_tag_exists(tag):
        errors.append(f"tag `{tag}` already exists locally")
    if tag in tags_on_remote:
        errors.append(f"tag `{tag}` already exists on `{remote}`")

    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print(f"CHANGELOG section for {tag} is new and links {url}")


if __name__ == "__main__":
    main()
