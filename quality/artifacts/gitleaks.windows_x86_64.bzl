
# buildifier: disable=attr-licenses
ARTIFACT = {
    "abi_floor": {
        "kernel": None,
        "libc": None,
        "libstdcxx": None,
    },
    "archive": {
        "format": "zip",
        "members": [
            {
                "is_executable": False,
                "mode": "0o644",
                "name": "LICENSE",
                "size": 1069,
            },
            {
                "is_executable": False,
                "mode": "0o644",
                "name": "README.md",
                "size": 29998,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "gitleaks.exe",
                "size": 22575104,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "gitleaks.exe",
    "executable_sha256": "17157e2ee8b76fc8b1d8bee607a250e34b8a8023c8bc81822d4b5ee4d78fcb7c",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/gitleaks/gitleaks/blob/v8.30.1/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [],
    "os": "windows",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "d29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e",
    "size": 8438883,
    "tool": "gitleaks",
    "upstream_version": "8.30.1",
    "url": "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_windows_x64.zip",
}
