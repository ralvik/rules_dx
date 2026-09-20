"""biome standalone artifact metadata (linux_arm64) -- GENERATED, do not edit.

Regenerate with: bazel run //quality/artifacts:update
"""

# buildifier: disable=attr-licenses  # ARTIFACT licenses key is SPDX data, not a rule attr
ARTIFACT = {
    "abi_floor": {
        "kernel": "3.7.0",
        "libc": "2.30",
        "libstdcxx": None,
    },
    "archive": {
        "format": "none",
    },
    "checksum_source": "maintainer-established byte identity (upstream publishes no asset digests)",
    "cpu": "arm64",
    "delivery_class": "standalone",
    "executable": "biome-linux-arm64",
    "executable_sha256": "4c1c9908e5cfd5d327e4ac3205baa13b1d3dfd24f184ed289a0c0ef1b2e0274f",
    "interpreter": "/lib/ld-linux-aarch64.so.1",
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
    "sha256": "4c1c9908e5cfd5d327e4ac3205baa13b1d3dfd24f184ed289a0c0ef1b2e0274f",
    "size": 57998768,
    "tool": "biome",
    "upstream_version": "2.5.12",
    "url": "https://github.com/biomejs/biome/releases/download/@biomejs/biome@2.5.12/biome-linux-arm64",
}
