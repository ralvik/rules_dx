#!/usr/bin/env python3
"""SPDX 2.3 generator for `sbom_release` (issue #318).

Hermetic replacement for the host `sha256sum` + `python3` heredoc:
computes the artifact digest with `hashlib` via the managed Python
3.12 toolchain and emits deterministic SPDX 2.3 JSON. No host tools.

Usage: sbom_spdx_gen.py <artifact> <out> <package_name> <supplier>
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
    if len(argv) != 5:
        print(
            "usage: sbom_spdx_gen.py <artifact> <out> <package> <supplier>",
            file=sys.stderr,
        )
        return 1
    src, out, pkg, sup = argv[1], argv[2], argv[3], argv[4]
    digest = sha256_file(src)
    base = os.path.basename(src)
    doc = {
        "SPDXID": "SPDXRef-DOCUMENT",
        "creationInfo": {
            "created": "1970-01-01T00:00:00Z",
            "creators": ["Tool: rules_dx-sbom-1.0"],
        },
        "dataLicense": "CC0-1.0",
        "documentNamespace": (
            "https://github.com/ralvik/rules_dx/releases/" + base + "-" + digest
        ),
        "files": [
            {
                "SPDXID": "SPDXRef-File",
                "checksums": [
                    {"algorithm": "SHA256", "checksumValue": digest}
                ],
                "fileName": base,
            }
        ],
        "name": pkg + "-" + base,
        "packages": [
            {
                "SPDXID": "SPDXRef-Package",
                "checksums": [
                    {"algorithm": "SHA256", "checksumValue": digest}
                ],
                "downloadLocation": "NOASSERTION",
                "externalRefs": [
                    {
                        "referenceCategory": "PACKAGE-MANAGER",
                        "referenceLocator": "pkg:generic/" + pkg + "@" + digest,
                        "referenceType": "purl",
                    }
                ],
                "filesAnalyzed": False,
                "name": pkg,
                "supplier": "Organization: " + sup,
                "verificationCode": {"packageVerificationCodeValue": digest},
            }
        ],
        "spdxVersion": "SPDX-2.3",
    }
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(doc, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
