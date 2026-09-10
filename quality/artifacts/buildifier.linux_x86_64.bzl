"""buildifier standalone artifact metadata (linux_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
Release: https://github.com/bazel-contrib/buildtools/releases/tag/v8.5.1
"""

# buildifier: disable=attr-licenses  # ARTIFACT licenses key is SPDX data, not a rule attr
ARTIFACT = {
    "abi_floor": {
        "kernel": None,
        "libc": None,
        "libstdcxx": None,
    },
    "archive": {
        "format": "none",
    },
    "checksum_source": "maintainer-established byte identity (upstream publishes no asset digests)",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "buildifier-linux-amd64",
    "executable_sha256": "887377fc64d23a850f4d18a077b5db05b19913f4b99b270d193f3c7334b5a9a7",
    "interpreter": None,
    "licenses": [
        {
            "name": "Apache-2.0",
            "source": "https://github.com/bazel-contrib/buildtools/blob/v8.5.1/LICENSE",
        },
    ],
    "linkage": "static",
    "needed_shared_libraries": [],
    "os": "linux",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "887377fc64d23a850f4d18a077b5db05b19913f4b99b270d193f3c7334b5a9a7",
    "size": 7757842,
    "tool": "buildifier",
    "upstream_version": "8.5.1",
    "url": "https://github.com/bazel-contrib/buildtools/releases/download/v8.5.1/buildifier-linux-amd64",
}
