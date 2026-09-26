
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
                "mode": "0o755",
                "name": "ruff-aarch64-apple-darwin",
                "size": 0,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "ruff-aarch64-apple-darwin/ruff",
                "size": 21582704,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "arm64",
    "delivery_class": "standalone",
    "executable": "ruff-aarch64-apple-darwin/ruff",
    "executable_sha256": "61da2ccac4b58c298f35544a59d2f52ab6bc4a8eacbe866edf7d20993206c569",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/astral-sh/ruff/blob/0.16.7/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "80221a5e0b1ae29262a74496f2ad1380c1ab52b3edd8cee13ec76d8acff406ca",
    "size": 9344878,
    "tool": "ruff",
    "upstream_version": "0.16.7",
    "url": "https://github.com/astral-sh/ruff/releases/download/0.16.7/ruff-aarch64-apple-darwin.tar.gz",
}
