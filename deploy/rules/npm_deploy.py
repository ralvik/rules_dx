#!/usr/bin/env python3
"""Local npm feed deployer for `npm_deploy`.

Verifies the hermetic pack tarball against its feed JSON, copies both
to the output directory, and assembles a folder feed. Live `npm
publish --access public --provenance` runs only with explicit env plus
owner approval; the default publishes nothing and needs no network.

Usage: npm_deploy.py [outdir]
"""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.parse


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _manifest_entries(manifest):
    entries = []
    with open(manifest, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            parts = line.split(" ", 1)
            if len(parts) != 2:
                continue
            entries.append((parts[0], parts[1]))
    return entries


def _find_in_manifest(manifest, predicate):
    return [real for rloc, real in _manifest_entries(manifest) if predicate(rloc)]


def _find_in_dir(root, predicate):
    found = []
    for dirpath, _, filenames in os.walk(root):
        for name in filenames:
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, root).replace(os.sep, "/")
            if predicate(rel):
                found.append(full)
    return found


def find_pack_inputs():
    feeds = []
    manifest = os.environ.get("RUNFILES_MANIFEST_FILE", "")
    if manifest and os.path.isfile(manifest):
        feeds = _find_in_manifest(manifest, lambda r: r.endswith(".feed.json"))
    runfiles = os.environ.get("RUNFILES_DIR", "")
    if not feeds and runfiles and os.path.isdir(runfiles):
        feeds = _find_in_dir(runfiles, lambda r: r.endswith(".feed.json"))
    if len(feeds) != 1:
        raise ValueError(
            "want exactly one .feed.json in runfiles, found " + str(len(feeds))
        )
    feed_path = feeds[0]
    with open(feed_path, encoding="utf-8") as f:
        feed = json.load(f)
    tarball = feed.get("tarball", "")
    if not tarball:
        raise ValueError("feed JSON is missing 'tarball'")
    tgzs = []
    if manifest and os.path.isfile(manifest):
        tgzs = _find_in_manifest(
            manifest,
            lambda r: r == tarball or r.endswith("/" + tarball),
        )
    if not tgzs and runfiles and os.path.isdir(runfiles):
        tgzs = _find_in_dir(
            runfiles,
            lambda r: r == tarball or r.endswith("/" + tarball),
        )
    if len(tgzs) != 1:
        raise ValueError(
            "want exactly one '" + tarball + "' in runfiles, found " + str(len(tgzs))
        )
    return feed_path, tgzs[0]


def verify_pack(tgz_path, feed_path):
    with open(feed_path, encoding="utf-8") as f:
        feed = json.load(f)
    for key in ("name", "tag", "registry", "tarball", "sha256", "files"):
        if key not in feed:
            raise ValueError("feed JSON is missing '" + key + "'")
    if type(feed["files"]) is not list:
        raise ValueError("feed JSON 'files' must be a list")
    expected = feed["sha256"]
    actual = sha256_file(tgz_path)
    if expected != actual:
        raise ValueError(
            "checksum mismatch for "
            + tgz_path
            + " (expected "
            + expected
            + ", actual "
            + actual
            + ")"
        )
    with tarfile.open(tgz_path, "r:gz") as tf:
        members = tf.getnames()
    if members != list(feed["files"]):
        raise ValueError("pack members mismatch for " + tgz_path + ": " + str(members))
    return feed


def _live_publish(tgz_path, feed):
    token = os.environ.get("NPM_TOKEN", "")
    if not token:
        print(
            "npm_deploy: live publish needs NPM_TOKEN plus explicit"
            " owner approval; publishing nothing",
            file=sys.stderr,
        )
        return 1
    tag = feed["tag"]
    registry = feed["registry"]
    cmd = [
        "npm",
        "publish",
        tgz_path,
        "--tag",
        tag,
        "--access",
        "public",
        "--provenance",
        "--registry",
        registry,
    ]
    if os.environ.get("NPM_PUBLISH_DRY_RUN", "") == "1":
        print("npm_deploy: dry run (NPM_PUBLISH_DRY_RUN=1); would publish:")
        print("  package: " + str(feed["name"]))
        print("  tag: " + str(tag))
        print("  command: " + " ".join(cmd))
        return 0
    if shutil.which("npm") is None:
        print("npm_deploy: 'npm' CLI not found on PATH", file=sys.stderr)
        return 1
    host = urllib.parse.urlparse(registry).netloc
    if not host:
        print("npm_deploy: invalid registry '" + registry + "'", file=sys.stderr)
        return 1
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".npmrc", delete=False, encoding="utf-8"
    ) as tmp:
        tmp.write("//" + host + "/:_authToken=" + token + "\n")
        npmrc = tmp.name
    try:
        result = subprocess.run(cmd + ["--userconfig", npmrc])
        return result.returncode
    finally:
        try:
            os.unlink(npmrc)
        except OSError:
            pass


def main(argv):
    if len(argv) > 1:
        outdir = argv[1]
    elif os.environ.get("BUILD_WORKSPACE_DIRECTORY", ""):
        outdir = os.environ["BUILD_WORKSPACE_DIRECTORY"]
    else:
        outdir = os.getcwd()
    os.makedirs(outdir, exist_ok=True)
    try:
        feed_path, tgz_path = find_pack_inputs()
        feed = verify_pack(tgz_path, feed_path)
    except (OSError, ValueError) as e:
        print("npm_deploy: " + str(e), file=sys.stderr)
        return 1
    tgz_base = os.path.basename(tgz_path)
    feed_base = os.path.basename(feed_path)
    stem = tgz_base[:-4] if tgz_base.endswith(".tgz") else tgz_base
    feed_dir = os.path.join(outdir, stem + "-feed")
    os.makedirs(feed_dir, exist_ok=True)
    shutil.copy(tgz_path, os.path.join(outdir, tgz_base))
    shutil.copy(feed_path, os.path.join(outdir, feed_base))
    shutil.copy(tgz_path, os.path.join(feed_dir, tgz_base))
    shutil.copy(feed_path, os.path.join(feed_dir, feed_base))
    profile = os.environ.get("DX_PROFILE", "<unset>")
    print(
        "npm_deploy: released "
        + str(feed["name"])
        + " (tag "
        + str(feed["tag"])
        + ", profile: "
        + profile
        + ")"
    )
    print("  tarball: " + os.path.join(outdir, tgz_base))
    print("  feed: " + feed_dir)
    if os.environ.get("NPM_PUBLISH_LIVE", "") == "1":
        return _live_publish(os.path.join(outdir, tgz_base), feed)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
