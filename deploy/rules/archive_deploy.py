#!/usr/bin/env python3
"""Local release stager."""

import hashlib
import os
import shutil
import sys

APP_RLOC = "@@APP_RLOC@@"
TARBALL_RLOC = "@@TARBALL_RLOC@@"
CHECKSUM_RLOC = "@@CHECKSUM_RLOC@@"


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def stage_release(app_src, tarball_src, checksum_src, outdir):
    app_name = os.path.basename(app_src)
    with open(checksum_src, encoding="utf-8") as f:
        expected = f.read().split()[0]
    actual = sha256_file(tarball_src)
    if expected != actual:
        raise RuntimeError(
            "archive_deploy: checksum mismatch for "
            + tarball_src
            + " (expected "
            + expected
            + ", actual "
            + actual
            + ")"
        )
    os.makedirs(outdir, exist_ok=True)
    tarball_dest = os.path.join(outdir, os.path.basename(tarball_src))
    checksum_dest = os.path.join(outdir, os.path.basename(checksum_src))
    shutil.copyfile(tarball_src, tarball_dest)
    shutil.copyfile(checksum_src, checksum_dest)
    return app_name, tarball_dest, checksum_dest


def _resolve_runfiles(rloc):
    from python.runfiles import Runfiles

    r = Runfiles.Create()
    path = r.Rlocation(rloc)
    if not path or not os.path.exists(path):
        raise RuntimeError("archive_deploy: runfile not found for '" + rloc + "'")
    return path


def main(argv):
    if len(argv) > 2:
        print(
            "archive_deploy: this deploy target takes at most an output directory; "
            "the release is exactly the files pinned at analysis time",
            file=sys.stderr,
        )
        return 1
    if len(argv) == 2:
        outdir = argv[1]
    else:
        outdir = os.environ.get("BUILD_WORKSPACE_DIRECTORY", os.getcwd())
    os.makedirs(outdir, exist_ok=True)

    app = _resolve_runfiles(APP_RLOC)
    tarball = _resolve_runfiles(TARBALL_RLOC)
    checksum = _resolve_runfiles(CHECKSUM_RLOC)

    try:
        app_name, tarball_dest, checksum_dest = stage_release(
            app, tarball, checksum, outdir
        )
    except (OSError, RuntimeError, IndexError) as e:
        print(str(e), file=sys.stderr)
        return 1
    print(
        "archive_deploy: released "
        + app_name
        + " (profile: "
        + os.environ.get("DX_PROFILE", "<unset>")
        + ")"
    )
    print("  tarball:  " + tarball_dest)
    print("  checksum: " + checksum_dest)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
