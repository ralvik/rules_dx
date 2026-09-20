"""ty standalone artifact metadata (macos_arm64) -- GENERATED, do not edit.

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
                "name": "ty-aarch64-apple-darwin",
                "size": 0,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "ty-aarch64-apple-darwin/ty",
                "size": 24981248,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "arm64",
    "delivery_class": "standalone",
    "executable": "ty-aarch64-apple-darwin/ty",
    "executable_sha256": "9b09467ea3db2d4e507c0f420fa1fb7e9d7099969310fdddf7a293bdad6baa93",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/astral-sh/ty/blob/0.0.80/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "745ad7e6e7aa410c50216488cacd25e2612f123dec535ec5408ae0aad4c3544d",
    "size": 12478461,
    "tool": "ty",
    "upstream_version": "0.0.80",
    "url": "https://github.com/astral-sh/ty/releases/download/0.0.80/ty-aarch64-apple-darwin.tar.gz",
}
