"""taplo standalone artifact metadata (linux_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
Release: https://github.com/tamasfe/taplo/releases/tag/0.10.0
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
        "member": "taplo-x86_64",
        "member_sha256": "dad2faf6377d2daa4f4fabf459fe7ccfb98a5448f0d4bca8270ca9acb0409bfe",
        "member_size": 12647136,
    },
    "checksum_source": "maintainer-established byte identity (upstream publishes no asset digests)",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "taplo-x86_64",
    "executable_sha256": "dad2faf6377d2daa4f4fabf459fe7ccfb98a5448f0d4bca8270ca9acb0409bfe",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/tamasfe/taplo/blob/0.10.0/LICENSE",
        },
    ],
    "linkage": "static-pie",
    "needed_shared_libraries": [],
    "os": "linux",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "8fe196b894ccf9072f98d4e1013a180306e17d244830b03986ee5e8eabeb6156",
    "size": 5116068,
    "tool": "taplo",
    "upstream_version": "0.10.0",
    "url": "https://github.com/tamasfe/taplo/releases/download/0.10.0/taplo-linux-x86_64.gz",
}
