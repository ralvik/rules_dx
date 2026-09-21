"""gitleaks standalone artifact metadata (linux_arm64) -- GENERATED, do not edit.

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
                "name": "gitleaks",
                "size": 20775096,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "arm64",
    "delivery_class": "standalone",
    "executable": "gitleaks",
    "executable_sha256": "00e91bbe655bd7c47753e8cfe61cb76ea1a5d7e7702fe161ee40102b46b3823b",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/gitleaks/gitleaks/blob/v8.30.1/LICENSE",
        },
    ],
    "linkage": "static",
    "needed_shared_libraries": [],
    "os": "linux",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080",
    "size": 7601421,
    "tool": "gitleaks",
    "upstream_version": "8.30.1",
    "url": "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_arm64.tar.gz",
}
