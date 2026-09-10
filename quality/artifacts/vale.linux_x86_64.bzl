"""vale standalone artifact metadata (linux_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
Release: https://github.com/vale-cli/vale/releases/tag/v3.20.0
"""

ARTIFACT = {
    "schema_version": 1,
    "tool": "vale",
    "upstream_version": "3.20.0",
    "delivery_class": "standalone",
    "os": "linux",
    "cpu": "x86_64",
    "url": "https://github.com/vale-cli/vale/releases/download/v3.20.0/vale_3.20.0_Linux_64-bit.tar.gz",
    "sha256": "f59e7030c5d4ace6cf915497d0d076a1699d61e876142765963237e6867c9712",
    "size": 11736086,
    "checksum_source": "upstream published checksums file",
    "archive": {
        "format": "tar.gz",
        "members": [
            {
                "name": "LICENSE",
                "size": 1068,
                "mode": "0o644",
                "is_executable": False,
            },
            {
                "name": "README.md",
                "size": 9722,
                "mode": "0o644",
                "is_executable": False,
            },
            {
                "name": "vale",
                "size": 45048160,
                "mode": "0o755",
                "is_executable": True,
            },
        ],
    },
    "executable": "vale",
    "executable_sha256": "17bd39892f5c2e2f62c72d85be47facd83c21d39597154996134d15e34af4428",
    "linkage": "dynamic",
    "interpreter": "/lib64/ld-linux-x86-64.so.2",
    "needed_shared_libraries": [
        "libc.so.6",
        "libgcc_s.so.1",
        "libpthread.so.0",
        "libresolv.so.2",
        "libstdc++.so.6",
    ],
    "abi_floor": {
        "kernel": "3.2.0",
        "libc": "2.17",
        "libstdcxx": "3.4",
    },
    "runtime_files": [],
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/vale-cli/vale/blob/v3.20.0/LICENSE",
        },
    ],
}
