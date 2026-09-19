#!/usr/bin/env python3
"""SLSA provenance generator for `sbom_release` (issue #318).

Hermetic replacement for the host `sha256sum` + `python3` heredoc:
computes the artifact digest with `hashlib` via the managed Python
3.12 toolchain and emits a deterministic in-toto Statement v1 with
the SLSA v1 predicate. No host tools.

Usage: sbom_prov_gen.py <artifact> <out> <builder_id>
"""

import hashlib
import json
import os
import sys


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main(argv):
    if len(argv) != 4:
        print(
            "usage: sbom_prov_gen.py <artifact> <out> <builder_id>",
            file=sys.stderr,
        )
        return 1
    src, out, builder = argv[1], argv[2], argv[3]
    digest = sha256_file(src)
    base = os.path.basename(src)
    stmt = {
        "_type": "https://in-toto.io/Statement/v1",
        "predicate": {
            "buildDefinition": {
                "buildType": "https://github.com/ralvik/rules_dx/release@v1",
                "externalParameters": {"artifact": base},
            },
            "runDetails": {
                "builder": {"id": builder},
                "metadata": {"invocationId": "dry-run"},
            },
        },
        "predicateType": "https://slsa.dev/provenance/v1",
        "subject": [{"digest": {"sha256": digest}, "name": base}],
    }
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(stmt, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
