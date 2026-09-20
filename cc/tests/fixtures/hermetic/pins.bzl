"""C/C++ sha256-integrity plus no-system-package pins (issue #484).

Contract: `docs/product/support-matrix.md#provisional-default-dependency-locks`.
No ecosystem lockfile; every http_archive carries sha256/integrity, system pkgs rejected.
"""

# Hash authority: every http_archive carries sha256 or integrity.
CC_HASH_ATTR = "sha256"
CC_HASH_ALT = "integrity"

# Lock authority: committed BCR lock carries integrity for the pinned module.
CC_LOCK_FILE = "//:MODULE.bazel.lock"
CC_LOCK_INTEGRITY = "registryFileHashes"
CC_PINNED_MODULE = "googletest 1.18.0"

# Depcheck authority: manifest plus lock pair with per-archive sha256.
CC_DEPCHECK_MANIFEST = "tools/depcheck/testdata/cc/ok_used/cc_deps.toml"
CC_DEPCHECK_LOCK = "tools/depcheck/testdata/cc/ok_used/cc_lock.json"

# Live proof labels (wrapper consumers over the hermetic toolchain).
CC_FIXTURE_HELLO = "//cc/tests/fixtures/hello:hello_test"
CC_FIXTURE_GTEST = "//cc/tests/fixtures/googletest:greeter_test"

# Rejected: system packages plus hash-less archives.
CC_REJECTED = "system packages rejected: no host apt/brew, no /usr paths, no local config; every http_archive carries sha256/integrity"
