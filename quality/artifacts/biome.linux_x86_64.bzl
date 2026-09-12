"""biome standalone artifact metadata (linux_x86_64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
Release: https://github.com/biomejs/biome/releases/tag/@biomejs/biome@2.5.12
"""

# buildifier: disable=attr-licenses  # ARTIFACT licenses key is SPDX data, not a rule attr
ARTIFACT = {
    "abi_floor": {
        "kernel": "3.2.0",
        "libc": "2.30",
        "libstdcxx": None,
    },
    "archive": {
        "format": "none",
    },
    "checksum_source": "maintainer-established byte identity (upstream publishes no asset digests)",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "biome-linux-x64",
    "executable_sha256": "e2475688799c9e78dd25ba5cf676676ffe74caf182a35082b1d22039151fdf63",
    "interpreter": "/lib64/ld-linux-x86-64.so.2",
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/biomejs/biome/blob/main/LICENSE-MIT",
        },
        {
            "name": "Apache-2.0",
            "source": "https://github.com/biomejs/biome/blob/main/LICENSE-APACHE",
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
    "sha256": "e2475688799c9e78dd25ba5cf676676ffe74caf182a35082b1d22039151fdf63",
    "size": 63652488,
    "tool": "biome",
    "upstream_version": "2.5.12",
    "url": "https://github.com/biomejs/biome/releases/download/@biomejs/biome@2.5.12/biome-linux-x64",
}
