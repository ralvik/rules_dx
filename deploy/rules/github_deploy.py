#!/usr/bin/env python3
"""Draft-only GitHub Release stager plus gated publisher for `github_deploy`.

Hermetic local default stages the pinned assets into a local release
directory plus a `would-run.txt` manifest (`gh release create <tag>
<assets...> --draft --verify-tag`) and verifies bytes via sha256; the
live `gh release create --draft --verify-tag` path runs only with
explicit env plus owner approval and never by default. Used as an `expand_template` template
per deploy instance (placeholders below) and as a `py_library` for
`py_test`.
"""

import hashlib
import os
import shutil
import subprocess
import sys

# Per-instance pins expanded by the `github_deploy` launcher rule. The
# checked-in placeholders keep this file importable for `py_test`, which
# exercises `build_staging` directly without touching these constants.
ASSET_RLOCS_STR = "@@ASSET_RLOCS@@"
DEPLOY_TAG = "@@TAG@@"
DEPLOY_NAME = "@@DEPLOY_NAME@@"

DRY_RUN_ENV = "GH_RELEASE_DRY_RUN"
LIVE_ENV = "GH_RELEASE_LIVE"
APPROVED_ENV = "GH_RELEASE_APPROVED"
PLACEHOLDER_TAG = "v0.0.0-dryrun"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def release_command(tag, asset_bases):
    parts = ["gh", "release", "create", tag] + list(asset_bases)
    parts += ["--draft", "--verify-tag"]
    return " ".join(parts)


def build_staging(asset_srcs, outdir, deploy_name, tag):
    """Copies pinned assets into a local release dir and writes the manifest.

    Creates `<outdir>/<deploy-name>-release/` holding each asset by
    basename plus `would-run.txt` with the `gh release create <tag>
    <assets...> --draft --verify-tag` line and per-asset sha256 lines.
    Verifies bytes via sha256 and returns the staging directory.
    """
    if not asset_srcs:
        raise ValueError("github release: need at least one asset")
    if not deploy_name:
        raise ValueError("github release: need a non-empty deploy name")
    if not tag:
        raise ValueError("github release: need a non-empty tag")
    staging = os.path.join(outdir, deploy_name + "-release")
    os.makedirs(staging, exist_ok=True)
    entries = []
    for src in sorted(asset_srcs):
        base = os.path.basename(src)
        if not base:
            raise ValueError("github release: empty basename for '" + src + "'")
        dest = os.path.join(staging, base)
        shutil.copyfile(src, dest)
        want = sha256_file(src)
        got = sha256_file(dest)
        if want != got:
            raise RuntimeError(
                "github release: byte mismatch for "
                + base
                + " (expected sha256 "
                + want
                + ", got "
                + got
                + ")"
            )
        entries.append((base, want))
    lines = [
        "# would-run manifest for github_deploy (local default publishes nothing)",
        release_command(tag, [base for base, _ in sorted(entries)]),
    ]
    for base, digest in sorted(entries):
        lines.append("asset-sha256: " + digest + "  " + base)
    with open(
        os.path.join(staging, "would-run.txt"), "w", encoding="utf-8", newline="\n"
    ) as f:
        f.write("\n".join(lines) + "\n")
    return staging


def live_publish(tag, asset_srcs):
    """Creates one draft release via gh without creating or pushing tags."""
    cmd = ["gh", "release", "create", tag] + list(asset_srcs) + ["--draft", "--verify-tag"]
    subprocess.run(cmd, check=True)


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("github_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "github_deploy: this deploy target takes at most an output directory; "
            "the release is exactly the artifacts pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    rlocs = [p for p in ASSET_RLOCS_STR.split(";") if p]
    assets = [_resolve_runfiles(rloc) for rloc in rlocs]
    tag = DEPLOY_TAG

    if os.environ.get(LIVE_ENV) == "1":
        approved = os.environ.get(APPROVED_ENV, "")
        if tag == PLACEHOLDER_TAG:
            print(
                "github_deploy: live publish refuses placeholder tag "
                + PLACEHOLDER_TAG
                + "; push a real tag first",
                file=sys.stderr,
            )
            return 1
        if approved != "1":
            print(
                "github_deploy: live publish needs "
                + APPROVED_ENV
                + "=1 (owner approval); refusing",
                file=sys.stderr,
            )
            return 1
        if shutil.which("gh") is None:
            print("github_deploy: 'gh' CLI not found on PATH", file=sys.stderr)
            return 1
        live_publish(tag, assets)
        print("github_deploy: published draft " + tag)
        return 0

    staging = build_staging(assets, outdir, DEPLOY_NAME, tag)
    dry = os.environ.get(DRY_RUN_ENV, "") == "1"
    if dry:
        print(
            "github_deploy: dry run (GH_RELEASE_DRY_RUN=1); "
            "would create a draft release, publishing nothing:"
        )
    else:
        print("github_deploy: staged draft release, publishing nothing:")
    print("  tag: " + tag)
    for src in sorted(assets):
        print("  asset: " + os.path.basename(src) + " (" + src + ")")
    print("  command: " + release_command(tag, [os.path.basename(a) for a in sorted(assets)]))
    print("  staging: " + staging)
    print("  manifest: " + os.path.join(staging, "would-run.txt"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
