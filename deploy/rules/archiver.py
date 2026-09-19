#!/usr/bin/env python3
"""Deterministic tar.gz archiver for `archive_release` (issue #318).

Hermetic replacement for `tar -czhf`: reads one staged executable file
(dereferencing symlinks like `tar -h`), writes a single-member gzip
tarball with fixed metadata (mtime 0, uid/gid 0, no uname/gname) via
the managed Python 3.12 toolchain. No host `tar`, no timestamps, no
uid leakage; output bytes are deterministic given input bytes.

Usage: archiver.py <src> <out>
"""

import gzip
import io
import os
import sys
import tarfile


def main(argv):
    if len(argv) != 3:
        print("usage: archiver.py <src> <out>", file=sys.stderr)
        return 1
    src, out = argv[1], argv[2]
    with open(src, "rb") as f:
        data = f.read()
    name = os.path.basename(src)
    st = os.stat(src)
    mode = 0o755 if (st.st_mode & 0o111) else 0o644
    info = tarfile.TarInfo(name=name)
    info.size = len(data)
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    info.mode = mode
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.PAX_FORMAT) as tf:
        tf.addfile(info, io.BytesIO(data))
    raw = buf.getvalue()
    with open(out, "wb") as f:
        with gzip.GzipFile(
            filename="", mode="wb", compresslevel=9, mtime=0, fileobj=f
        ) as gz:
            gz.write(raw)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
