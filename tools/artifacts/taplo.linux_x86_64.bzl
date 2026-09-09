"""taplo standalone artifact metadata (linux_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //tools/artifacts:update
Release: https://github.com/tamasfe/taplo/releases/tag/0.10.0
"""

ARTIFACT = {
    "schema_version": 1,
    "tool": "taplo",
    "upstream_version": "0.10.0",
    "delivery_class": "standalone",
    "os": "linux",
    "cpu": "x86_64",
    "url": "https://github.com/tamasfe/taplo/releases/download/0.10.0/taplo-linux-x86_64.gz",
    "sha256": "8fe196b894ccf9072f98d4e1013a180306e17d244830b03986ee5e8eabeb6156",
    "size": 5116068,
    "checksum_source": "maintainer-established byte identity (upstream publishes no asset digests)",
    "archive": {
        "format": "gzip",
        "member": "taplo-x86_64",
        "member_sha256": "dad2faf6377d2daa4f4fabf459fe7ccfb98a5448f0d4bca8270ca9acb0409bfe",
        "member_size": 12647136,
    },
    "executable": "taplo-x86_64",
    "executable_sha256": "dad2faf6377d2daa4f4fabf459fe7ccfb98a5448f0d4bca8270ca9acb0409bfe",
    "linkage": "static-pie",
    "interpreter": None,
    "needed_shared_libraries": [],
    "abi_floor": {
        "kernel": None,
        "libc": None,
        "libstdcxx": None,
    },
    "runtime_files": [],
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/tamasfe/taplo/blob/0.10.0/LICENSE",
        },
    ],
}
