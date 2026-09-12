"""ty standalone artifact metadata (linux_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
Release: https://github.com/astral-sh/ty/releases/tag/0.0.80
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
                "name": "ty-x86_64-unknown-linux-gnu",
                "size": 0,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "ty-x86_64-unknown-linux-gnu/ty",
                "size": 28581056,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "ty-x86_64-unknown-linux-gnu/ty",
    "executable_sha256": "147dde31480eb80efb7ea4646486505673fbd254c0239bbc95785467c3defe39",
    "interpreter": "/lib64/ld-linux-x86-64.so.2",
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/astral-sh/ty/blob/0.0.80/LICENSE",
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
    "sha256": "6c4142846197f39a3ac963a01512126eea2c01813c2fb4cd0ce7356fc1c9304b",
    "size": 13300460,
    "tool": "ty",
    "upstream_version": "0.0.80",
    "url": "https://github.com/astral-sh/ty/releases/download/0.0.80/ty-x86_64-unknown-linux-gnu.tar.gz",
}
