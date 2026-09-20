"""taplo standalone artifact metadata (macos_x86_64) -- GENERATED, do not edit.

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
        "format": "gzip",
        "member": "taplo",
        "member_sha256": "9fd7a2872ea154df61a2c7e9ca69fc19ac08e29f2e2dc2f866e299bdc789c1a1",
        "member_size": 11367780,
    },
    "checksum_source": "maintainer-established byte identity (upstream publishes no asset digests)",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "taplo",
    "executable_sha256": "9fd7a2872ea154df61a2c7e9ca69fc19ac08e29f2e2dc2f866e299bdc789c1a1",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/tamasfe/taplo/blob/0.10.0/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "898122cde3a0b1cd1cbc2d52d3624f23338218c91b5ddb71518236a4c2c10ef2",
    "size": 4921954,
    "tool": "taplo",
    "upstream_version": "0.10.0",
    "url": "https://github.com/tamasfe/taplo/releases/download/0.10.0/taplo-darwin-x86_64.gz",
}
