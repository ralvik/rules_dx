#!/usr/bin/env python3
"""Deterministic npm pack generator."""

import gzip
import hashlib
import io
import json
import os
import sys
import tarfile


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def create_pack(package, tag, registry, tgz_out, feed_out, srcs):
    members = {}
    for src in srcs:
        with open(src, "rb") as f:
            data = f.read()
        base = os.path.basename(src)
        member = "package/" + base
        if member in members:
            raise ValueError("duplicate pack member '" + member + "'")
        st = os.stat(src)
        mode = 0o755 if (st.st_mode & 0o111) else 0o644
        members[member] = (data, mode)
    names = sorted(members.keys())
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.PAX_FORMAT) as tf:
        for member in names:
            data, mode = members[member]
            info = tarfile.TarInfo(name=member)
            info.size = len(data)
            info.mtime = 0
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            info.mode = mode
            tf.addfile(info, io.BytesIO(data))
    raw = buf.getvalue()
    with open(tgz_out, "wb") as f:
        with gzip.GzipFile(
            filename="", mode="wb", compresslevel=9, mtime=0, fileobj=f
        ) as gz:
            gz.write(raw)
    digest = sha256_file(tgz_out)
    feed = {
        "files": names,
        "name": package,
        "registry": registry,
        "sha256": digest,
        "tag": tag,
        "tarball": os.path.basename(tgz_out),
    }
    with open(feed_out, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(feed, indent=2, sort_keys=True) + "\n")
    return digest


def main(argv):
    if len(argv) < 7:
        print(
            "usage: npm_pack.py <package> <tag> <registry>"
            " <tgz_out> <feed_out> <src...>",
            file=sys.stderr,
        )
        return 1
    package, tag, registry = argv[1], argv[2], argv[3]
    tgz_out, feed_out, srcs = argv[4], argv[5], argv[6:]
    try:
        create_pack(package, tag, registry, tgz_out, feed_out, srcs)
    except (OSError, ValueError) as e:
        print("npm_pack: " + str(e), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
