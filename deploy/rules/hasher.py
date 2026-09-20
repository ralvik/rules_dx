#!/usr/bin/env python3
"""sha256 checksum writer for `archive_release`.

Hermetic replacement for `sha256sum src > out`: computes the file
digest with `hashlib` via the managed Python 3.12 toolchain and writes
a `sha256sum`-compatible `<digest>  <basename>` line. No host
`sha256sum`/`shasum` probing.

Usage: hasher.py <src> <out>
"""

import hashlib
import os
import sys


def main(argv):
    if len(argv) != 3:
        print("usage: hasher.py <src> <out>", file=sys.stderr)
        return 1
    src, out = argv[1], argv[2]
    digest = hashlib.sha256()
    with open(src, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    base = os.path.basename(src)
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(digest.hexdigest() + "  " + base + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
