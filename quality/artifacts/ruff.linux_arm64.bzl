"""ruff standalone artifact metadata (linux_arm64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
"""

# buildifier: disable=attr-licenses  # ARTIFACT licenses key is SPDX data, not a rule attr
ARTIFACT = {
    "abi_floor": {
        "kernel": "3.7.0",
        "libc": "2.17",
        "libstdcxx": None,
    },
    "archive": {
        "format": "tar.gz",
        "members": [
            {
                "is_executable": False,
                "mode": "0o755",
                "name": "ruff-aarch64-unknown-linux-gnu",
                "size": 0,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "ruff-aarch64-unknown-linux-gnu/ruff",
                "size": 21968392,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "arm64",
    "delivery_class": "standalone",
    "executable": "ruff-aarch64-unknown-linux-gnu/ruff",
    "executable_sha256": "0e3c9829ec8e95bcd8a36f6523a1fe6c30ae30ccae31d9408ecc2e64d975768b",
    "interpreter": "/lib/ld-linux-aarch64.so.1",
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
    ],
    "os": "linux",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "1e06b11127c28387c8066da4ce6a617a84a359591be09dbf581fbb0c3e006239",
    "size": 9443710,
    "tool": "ruff",
    "upstream_version": "0.16.7",
    "url": "https://github.com/astral-sh/ruff/releases/download/0.16.7/ruff-aarch64-unknown-linux-gnu.tar.gz",
}
