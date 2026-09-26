"""vale standalone artifact metadata (macos_arm64) -- GENERATED, do not edit."""

# buildifier: disable=attr-licenses
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
                "size": 44378898,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "arm64",
    "delivery_class": "standalone",
    "executable": "vale",
    "executable_sha256": "048f05392a0b26f52ce8f49a03d1b19d4dcc73dc6ab77e2cd9a9e9418a4f25dc",
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
    "sha256": "6b32df1a7d7b2ab01c07c1c4dd5fd84d775ac7a8f4101084cb5cc7df76bdc65e",
    "size": 11300418,
    "tool": "vale",
    "upstream_version": "3.20.0",
    "url": "https://github.com/vale-cli/vale/releases/download/v3.20.0/vale_3.20.0_macOS_arm64.tar.gz",
}
