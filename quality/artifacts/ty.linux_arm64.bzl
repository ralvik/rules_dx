"""ty standalone artifact metadata (linux_arm64) -- GENERATED, do not edit."""

# buildifier: disable=attr-licenses
ARTIFACT = {
    "abi_floor": {
        "kernel": "3.7.0",
        "libc": "2.17",
        "libstdcxx": None,
    },
    "archive": {
        "format": "tar.gz",
        "members": [
            {
                "is_executable": False,
                "mode": "0o755",
                "name": "ty-aarch64-unknown-linux-gnu",
                "size": 0,
            },
            {
                "is_executable": True,
                "mode": "0o755",
                "name": "ty-aarch64-unknown-linux-gnu/ty",
                "size": 26368328,
            },
        ],
    },
    "checksum_source": "upstream published checksums file",
    "cpu": "arm64",
    "delivery_class": "standalone",
    "executable": "ty-aarch64-unknown-linux-gnu/ty",
    "executable_sha256": "48791915925a18e2009debfac679705fee95838d01085a018002d7841a68f283",
    "interpreter": "/lib/ld-linux-aarch64.so.1",
    "licenses": [
        {
            "name": "MIT",
            "source": "https://github.com/astral-sh/ty/blob/0.0.80/LICENSE",
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
    "sha256": "c8a21f89ddea64bc6f79de733cdd005d02df0b98da55dc6376681f5d418af16e",
    "size": 12525064,
    "tool": "ty",
    "upstream_version": "0.0.80",
    "url": "https://github.com/astral-sh/ty/releases/download/0.0.80/ty-aarch64-unknown-linux-gnu.tar.gz",
}
