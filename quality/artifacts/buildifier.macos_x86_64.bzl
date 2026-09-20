"""buildifier standalone artifact metadata (macos_x86_64) -- GENERATED, do not edit.

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
        "format": "none",
    },
    "checksum_source": "maintainer-established byte identity (upstream publishes no asset digests)",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "buildifier-darwin-amd64",
    "executable_sha256": "31de189e1a3fe53aa9e8c8f74a0309c325274ad19793393919e1ca65163ca1a4",
    "interpreter": None,
    "licenses": [
        {
            "name": "Apache-2.0",
            "source": "https://github.com/bazel-contrib/buildtools/blob/v8.5.1/LICENSE",
        },
    ],
    "linkage": "static",
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "31de189e1a3fe53aa9e8c8f74a0309c325274ad19793393919e1ca65163ca1a4",
    "size": 7737072,
    "tool": "buildifier",
    "upstream_version": "8.5.1",
    "url": "https://github.com/bazel-contrib/buildtools/releases/download/v8.5.1/buildifier-darwin-amd64",
}
