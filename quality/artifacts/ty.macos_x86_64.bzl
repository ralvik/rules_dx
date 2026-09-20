"""ty standalone artifact metadata (macos_x86_64) -- GENERATED, do not edit.

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
                "mode": "0o755",
                "name": "ty-x86_64-apple-darwin",
                "size": 0,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "ty-x86_64-apple-darwin/ty",
                "size": 27645264,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "x86_64",
    "delivery_class": "standalone",
    "executable": "ty-x86_64-apple-darwin/ty",
    "executable_sha256": "80062d6a95d20ea1ee0a95d1aa3e40dc5821bfcaf94347a78219911ed8151770",
    "interpreter": None,
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/astral-sh/ty/blob/0.0.80/LICENSE",
        },
    ],
    "linkage": "dynamic",
    "needed_shared_libraries": [],
    "os": "macos",
    "runtime_files": [],
    "schema_version": 1,
    "sha256": "3718c4ab92202e98bcede7c07ecfea1734c3398b7726efb092de4a073ee45d2c",
    "size": 12843954,
    "tool": "ty",
    "upstream_version": "0.0.80",
    "url": "https://github.com/astral-sh/ty/releases/download/0.0.80/ty-x86_64-apple-darwin.tar.gz",
}
