#!/usr/bin/env python3
"""Release helpers for the publishable workspace crates.

    crates.py version              print the shared workspace release version
    crates.py publishable          print the publishable package names
    crates.py unpublished VERSION  print the publishable packages whose VERSION
                                   is not yet in the crates.io index, logging
                                   each package it skips to stderr

`version` fails unless every publishable package carries the same version.
`unpublished` fails on any index error other than "crate not found", so a
network problem can never be mistaken for "not published yet".
"""

import json
import subprocess
import sys
import urllib.error
import urllib.request

INDEX = "https://index.crates.io"
USER_AGENT = "solverforge-release (https://github.com/SolverForge/solverforge)"


def publishable_packages():
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    packages = []
    for package in metadata["packages"]:
        registries = package.get("publish")
        if registries is None or "crates-io" in registries:
            packages.append(package)
    if not packages:
        sys.exit("no publishable workspace packages found")
    return packages


def workspace_version(packages):
    versions = {package["version"] for package in packages}
    if len(versions) != 1:
        listing = ", ".join(f"{p['name']} {p['version']}" for p in packages)
        sys.exit(f"publishable packages disagree on the release version: {listing}")
    return versions.pop()


def index_path(name):
    name = name.lower()
    if len(name) <= 2:
        return f"{len(name)}/{name}"
    if len(name) == 3:
        return f"3/{name[0]}/{name}"
    return f"{name[0:2]}/{name[2:4]}/{name}"


def published_versions(name):
    request = urllib.request.Request(
        f"{INDEX}/{index_path(name)}", headers={"User-Agent": USER_AGENT}
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = response.read().decode()
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return set()
        raise
    return {json.loads(line)["vers"] for line in body.splitlines() if line.strip()}


def main(argv):
    if len(argv) < 2:
        sys.exit(__doc__)
    command = argv[1]
    packages = publishable_packages()
    if command == "version" and len(argv) == 2:
        print(workspace_version(packages))
    elif command == "publishable" and len(argv) == 2:
        for package in packages:
            print(package["name"])
    elif command == "unpublished" and len(argv) == 3:
        version = argv[2]
        expected = workspace_version(packages)
        if version != expected:
            sys.exit(f"requested version {version} is not the workspace version {expected}")
        for package in packages:
            if version in published_versions(package["name"]):
                print(f"skip {package['name']} {version} (already on crates.io)", file=sys.stderr)
            else:
                print(package["name"])
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
