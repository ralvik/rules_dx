"""gitleaks standalone artifact metadata (macos_x86_64) -- GENERATED, do not edit.

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
                "size": 22398576,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "gitleaks",
    "executable_sha256": "cee01fea7173f1b779dff188e1c26ecbcb4027d394acc573b23aaf0be260e291",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/gitleaks/gitleaks/blob/v8.30.1/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "dfe101a4db2255fc85120ac7f3d25e4342c3c20cf749f2c20a18081af1952709",
    "size": 8359235,
    "tool": "gitleaks",
    "upstream_version": "8.30.1",
    "url": "https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_darwin_x64.tar.gz",
}
