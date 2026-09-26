"""Registry and hub inventory for the lock dialects."""

NPM_REGISTRY_HUBS = {
    "npm": "//:pnpm-lock.yaml",
    "npm_tools": "//quality/tools/javascript:pnpm-lock.yaml",
}

UV_REGISTRY_HUBS = {
    "pypi": [
        "//python/tests/fixtures/hello:uv.lock",
        "//quality/tools/python:uv.lock",
    ],
}

CRATE_HUBS = ["crates"]

LOCK_DIALECTS = ["cargo", "uv", "pnpm", "go", "maven", "paket"]
