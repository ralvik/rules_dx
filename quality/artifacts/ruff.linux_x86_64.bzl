"""ruff standalone artifact metadata (linux_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
Release: https://github.com/astral-sh/ruff/releases/tag/0.16.7
"""

# buildifier: disable=attr-licenses  # ARTIFACT licenses key is SPDX data, not a rule attr
ARTIFACT = {
    "abi_floor": {
        "kernel": "2.6.32",
        "libc": "2.17",
        "libstdcxx": None,
    },
    "archive": {
        "format": "tar.gz",
        "members": [
            {
                "is_executable": False,
                "mode": "0o755",
                "name": "ruff-x86_64-unknown-linux-gnu",
                "size": 0,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "ruff-x86_64-unknown-linux-gnu/ruff",
                "size": 24079648,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "ruff-x86_64-unknown-linux-gnu/ruff",
    "executable_sha256": "fef5ab1d39e0c8368a3ef2511f33083f372943ac76fe4bb2831f312d0cd9a5bd",
    "interpreter": "/lib64/ld-linux-x86-64.so.2",
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/astral-sh/ruff/blob/0.16.7/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [
        "libc.so.6",
        "libdl.so.2",
        "libgcc_s.so.1",
        "libm.so.6",
        "libpthread.so.0",
        "librt.so.1",
    ],
    "os": "linux",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "73894c7b7c9a53fd66ed715eb3a1ec65077f316328e377057a98bdb7fcba0326",
    "size": 9977676,
    "tool": "ruff",
    "upstream_version": "0.16.7",
    "url": "https://github.com/astral-sh/ruff/releases/download/0.16.7/ruff-x86_64-unknown-linux-gnu.tar.gz",
}
