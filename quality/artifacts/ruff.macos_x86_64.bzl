"""ruff standalone artifact metadata (macos_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
"""

# buildifier: disable=attr-licenses  # ARTIFACT licenses key is SPDX data, not a rule attr
ARTIFACT = {
    "abi_floor": {
        "kernel": None,
        "libc": None,
        "libstdcxx": None,
    },
    "archive": {
        "format": "tar.gz",
        "members": [
            {
                "is_executable": False,
                "mode": "0o755",
                "name": "ruff-x86_64-apple-darwin",
                "size": 0,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "ruff-x86_64-apple-darwin/ruff",
                "size": 23068456,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "ruff-x86_64-apple-darwin/ruff",
    "executable_sha256": "f2395291ca6da479e14cfd54b62aa00938eafbe0614e63c7ee6e588ef047db10",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/astral-sh/ruff/blob/0.16.7/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "6f3b98ec349f470b7efde5294d44e5171613ddaac3c251a918a7946b303d3ec2",
    "size": 9864923,
    "tool": "ruff",
    "upstream_version": "0.16.7",
    "url": "https://github.com/astral-sh/ruff/releases/download/0.16.7/ruff-x86_64-apple-darwin.tar.gz",
}
