#!/usr/bin/env python3
"""BCR source template generator for `bcr_check` (issue #318).

Hermetic replacement for the host `python3` heredoc: emits the
deterministic BCR `source.json` shape via the managed Python 3.12
toolchain. No host tools.

Usage: bcr_source_gen.py <out> <module_name> <version>
"""

import json
import sys


def main(argv):
    if len(argv) != 4:
        print(
            "usage: bcr_source_gen.py <out> <module> <version>",
            file=sys.stderr,
        )
        return 1
    out, mod, ver = argv[1], argv[2], argv[3]
    src = {
        "integrity": "<integrity-filled-at-release>",
        "strip_prefix": mod + "-" + ver,
        "url": (
            "https://github.com/ralvik/rules_dx/releases/download/v"
            + ver
            + "/"
            + mod
            + "-"
            + ver
            + ".tar.gz"
        ),
    }
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(src, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
