"""Registry and hub inventory for the lock dialects."""

# npm registry hubs (see //modules:js.bzl; authoritative plus private tool graph).
NPM_REGISTRY_HUBS = {
    "npm": "//:pnpm-lock.yaml",
    "npm_tools": "//quality/tools/javascript:pnpm-lock.yaml",
}

# uv registry hub (see //modules:python.bzl; single `pypi` hub).
UV_REGISTRY_HUBS = {
    "pypi": [
        "//python/tests/fixtures/hello:uv.lock",
        "//quality/tools/python:uv.lock",
    ],
}

# Crate hubs: single `crates` hub today over the manifest groups in
# //modules:rust.bzl. A future multi-hub split moves whole groups:
# fixtures to `crates_fixtures`, cli to `crates_cli`, deploy to
# `crates_deploy`, shard writers to `crates_shards`, shared to `crates_shared`.
CRATE_HUBS = ["crates"]

# Lock dialects owned by depcheck pin_consistency (see tools/depcheck/src/lib.rs).
LOCK_DIALECTS = ["cargo", "uv", "pnpm", "go", "maven", "paket"]
