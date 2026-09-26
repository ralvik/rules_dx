#!/usr/bin/env python3
"""Local folder feed builder."""

import hashlib
import os
import shutil
import subprocess
import sys

NUPKG_RLOC = "@@NUPKG_RLOC@@"
PACKAGE_ID = "@@PACKAGE_ID@@"
PACKAGE_VERSION = "@@PACKAGE_VERSION@@"
PACKAGE_SOURCE = "@@PACKAGE_SOURCE@@"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_feed(nupkg_src, outdir, package_id):
    if not nupkg_src or not nupkg_src.endswith(".nupkg"):
        raise ValueError(
            "nuget feed: want exactly one .nupkg source, got '" + str(nupkg_src) + "'"
        )
    feed = os.path.join(outdir, package_id + "-feed")
    os.makedirs(feed, exist_ok=True)
    base = os.path.basename(nupkg_src)
    dest = os.path.join(feed, base)
    shutil.copyfile(nupkg_src, dest)
    want = sha256_file(nupkg_src)
    got = sha256_file(dest)
    if want != got:
        raise RuntimeError(
            "nuget feed: byte mismatch for "
            + base
            + " (expected sha256 "
            + want
            + ", got "
            + got
            + ")"
        )
    return feed


def live_push(nupkg_src, source, api_key):
    cmd = [
        "dotnet",
        "nuget",
        "push",
        nupkg_src,
        "--source",
        source,
        "--api-key",
        api_key,
        "--skip-duplicate",
    ]
    subprocess.run(cmd, check=True)


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("nuget_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "nuget_deploy: this deploy target takes at most an output directory; "
            "the feed is exactly the file pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    nupkg = _resolve_runfiles(NUPKG_RLOC)

    if os.environ.get("NUGET_PUBLISH_LIVE") == "1":
        token = os.environ.get("NUGET_API_KEY", "")
        approved = os.environ.get("NUGET_PUBLISH_APPROVED", "")
        if PACKAGE_VERSION == "0.0.0":
            print(
                "nuget_deploy: live push refuses version 0.0.0; set a real version",
                file=sys.stderr,
            )
            return 1
        if not token:
            print(
                "nuget_deploy: live push needs NUGET_API_KEY plus explicit "
                "owner approval (NUGET_PUBLISH_APPROVED=1); refusing",
                file=sys.stderr,
            )
            return 1
        if approved != "1":
            print(
                "nuget_deploy: live push needs NUGET_PUBLISH_APPROVED=1 "
                "(owner approval); refusing",
                file=sys.stderr,
            )
            return 1
        live_push(nupkg, PACKAGE_SOURCE, token)
        print("nuget_deploy: pushed " + PACKAGE_ID + " " + PACKAGE_VERSION)
        return 0

    feed = build_feed(nupkg, outdir, PACKAGE_ID)
    print(
        "nuget_deploy: staged "
        + PACKAGE_ID
        + " "
        + PACKAGE_VERSION
        + " (profile: "
        + os.environ.get("DX_PROFILE", "<unset>")
        + ")"
    )
    print("  feed: " + feed)
    print("  nupkg: " + os.path.join(feed, os.path.basename(nupkg)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
