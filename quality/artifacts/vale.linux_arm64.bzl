"""vale standalone artifact metadata (linux_arm64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
"""

# buildifier: disable=attr-licenses  # ARTIFACT licenses key is SPDX data, not a rule attr
ARTIFACT = {
    "abi_floor": {
        "kernel": "3.7.0",
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
                "size": 43937760,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "arm64",
    "delivery_class": "standalone",
    "executable": "vale",
    "executable_sha256": "1d60b298df01b7709acd46967e99035c90d0fccaa59cd6bc3e4f973a9d7f00f1",
    "interpreter": "/lib/ld-linux-aarch64.so.1",
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
    "sha256": "d49e89479a40dbe6a6fe44a2963abef0ee39d89453f3c7100bdd1c5dd0708961",
    "size": 10969478,
    "tool": "vale",
    "upstream_version": "3.20.0",
    "url": "https://github.com/vale-cli/vale/releases/download/v3.20.0/vale_3.20.0_Linux_arm64.tar.gz",
}
