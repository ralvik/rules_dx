"""vale standalone artifact metadata (macos_x86_64) -- GENERATED, do not edit.

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
                "mode": "0o644",
                "name": "LICENSE",
                "size": 1068,
            },
            {
                "is_executable": False,
                "mode": "0o644",
                "name": "README.md",
                "size": 9722,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "vale",
                "size": 45108184,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "vale",
    "executable_sha256": "ef499e26dc9a5f0b3ca75b598bad8ee03f9d79a0c99af2e6e5fe7a774a648b24",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/vale-cli/vale/blob/v3.20.0/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "8d51cbe9ca6274fade890fc943b30dc564071dcb8fd8814abce2d0dca37fbba7",
    "size": 11725558,
    "tool": "vale",
    "upstream_version": "3.20.0",
    "url": "https://github.com/vale-cli/vale/releases/download/v3.20.0/vale_3.20.0_macOS_64-bit.tar.gz",
}
