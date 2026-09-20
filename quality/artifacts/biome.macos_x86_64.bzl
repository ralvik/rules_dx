"""biome standalone artifact metadata (macos_x86_64) -- GENERATED, do not edit.

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
    "executable": "biome-darwin-x64",
    "executable_sha256": "0680097ec839fadc82451634680e39b9b7e48cc4334ef920525850b4d8f54a8b",
    "interpreter": None,
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
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "0680097ec839fadc82451634680e39b9b7e48cc4334ef920525850b4d8f54a8b",
    "size": 58413368,
    "tool": "biome",
    "upstream_version": "2.5.12",
    "url": "https://github.com/biomejs/biome/releases/download/@biomejs/biome@2.5.12/biome-darwin-x64",
}
