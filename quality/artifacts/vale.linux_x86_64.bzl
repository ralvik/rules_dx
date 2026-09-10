"""vale standalone artifact metadata (linux_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
Release: https://github.com/vale-cli/vale/releases/tag/v3.20.0
"""

# buildifier: disable=attr-licenses  # ARTIFACT licenses key is SPDX data, not a rule attr
ARTIFACT = {
    "abi_floor": {
        "kernel": "3.2.0",
        "libc": "2.17",
        "libstdcxx": "3.4",
    },
    "archive": {
        "format": "tar.gz",
        "members": [
            {
                "is_executable": False,
                "mode": "0o644",
                "name": "LICENSE",
                "size": 1068,
            },
            {
                "is_executable": False,
                "mode": "0o644",
                "name": "README.md",
                "size": 9722,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "vale",
                "size": 45048160,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "vale",
    "executable_sha256": "17bd39892f5c2e2f62c72d85be47facd83c21d39597154996134d15e34af4428",
    "interpreter": "/lib64/ld-linux-x86-64.so.2",
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/vale-cli/vale/blob/v3.20.0/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [
        "libc.so.6",
        "libgcc_s.so.1",
        "libpthread.so.0",
        "libresolv.so.2",
        "libstdc++.so.6",
    ],
    "os": "linux",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "f59e7030c5d4ace6cf915497d0d076a1699d61e876142765963237e6867c9712",
    "size": 11736086,
    "tool": "vale",
    "upstream_version": "3.20.0",
    "url": "https://github.com/vale-cli/vale/releases/download/v3.20.0/vale_3.20.0_Linux_64-bit.tar.gz",
}
