#!/usr/bin/env python3
"""Local wheelhouse builder."""

import hashlib
import os
import shutil
import subprocess
import sys

WHEEL_RLOC = "@@WHEEL_RLOC@@"
SDIST_RLOC = "@@SDIST_RLOC@@"
DIST_NAME = "@@DIST_NAME@@"
REPOSITORY_URL = "@@REPOSITORY_URL@@"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_wheelhouse(wheel_src, sdist_src, outdir, dist_name):
    house = os.path.join(outdir, dist_name + "-wheelhouse")
    os.makedirs(house, exist_ok=True)
    entries = []
    for src in [p for p in (wheel_src, sdist_src) if p]:
        base = os.path.basename(src)
        dest = os.path.join(house, base)
        shutil.copyfile(src, dest)
        want = sha256_file(src)
        got = sha256_file(dest)
        if want != got:
            raise RuntimeError(
                "pypi wheelhouse: byte mismatch for "
                + base
                + " (expected sha256 "
                + want
                + ", got "
                + got
                + ")"
            )
        entries.append((base, want))
    index_dir = os.path.join(house, "simple-index", dist_name)
    os.makedirs(index_dir, exist_ok=True)
    lines = [
        "<!DOCTYPE html>",
        "<html><body>",
    ]
    for base, digest in sorted(entries):
        lines.append(
            '<a href="../../' + base + "#sha256=" + digest + '">' + base + "</a><br/>"
        )
    lines.append("</body></html>")
    with open(
        os.path.join(index_dir, "index.html"), "w", encoding="utf-8", newline="\n"
    ) as f:
        f.write("\n".join(lines) + "\n")
    return house


def minimal_upload_env(extra):
    keep = (
        "HOME",
        "LANG",
        "LC_ALL",
        "PATH",
        "TMPDIR",
        "USER",
        "LOGNAME",
        "SystemRoot",
        "SystemDrive",
        "PATHEXT",
    )
    env = {key: os.environ[key] for key in keep if key in os.environ}
    env.update(extra)
    return env


def live_upload(wheel_src, sdist_src, repository_url, token):
    files = [wheel_src] + ([sdist_src] if sdist_src else [])
    env = minimal_upload_env(
        {"TWINE_USERNAME": "__token__", "TWINE_PASSWORD": token}
    )
    cmd = [
        "twine",
        "upload",
        "--non-interactive",
        "--repository-url",
        repository_url,
    ] + files
    subprocess.run(cmd, env=env, check=True)


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("pypi_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "pypi_deploy: this deploy target takes at most an output directory; "
            "the wheelhouse is exactly the files pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    wheel = _resolve_runfiles(WHEEL_RLOC)
    sdist = _resolve_runfiles(SDIST_RLOC) if SDIST_RLOC else None

    if os.environ.get("PYPI_PUBLISH_LIVE") == "1":
        token = os.environ.get("PYPI_API_TOKEN", "")
        approved = os.environ.get("PYPI_PUBLISH_APPROVED", "")
        if not token:
            print(
                "pypi_deploy: live upload needs PYPI_API_TOKEN plus explicit "
                "owner approval (PYPI_PUBLISH_APPROVED=1); refusing",
                file=sys.stderr,
            )
            return 1
        if approved != "1":
            print(
                "pypi_deploy: live upload needs PYPI_PUBLISH_APPROVED=1 "
                "(owner approval); refusing",
                file=sys.stderr,
            )
            return 1
        live_upload(wheel, sdist, REPOSITORY_URL, token)
        print(
            "pypi_deploy: uploaded " + os.path.basename(wheel) + " to " + REPOSITORY_URL
        )
        return 0

    house = build_wheelhouse(wheel, sdist, outdir, DIST_NAME)
    print(
        "pypi_deploy: staged "
        + DIST_NAME
        + " (profile: "
        + os.environ.get("DX_PROFILE", "<unset>")
        + ")"
    )
    print("  wheelhouse: " + house)
    print("  index: " + os.path.join(house, "simple-index", DIST_NAME, "index.html"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
