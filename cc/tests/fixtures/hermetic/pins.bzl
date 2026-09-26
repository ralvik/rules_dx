CC_HASH_ATTR = "sha256"
CC_HASH_ALT = "integrity"

CC_LOCK_FILE = "//:MODULE.bazel.lock"
CC_LOCK_INTEGRITY = "registryFileHashes"
CC_PINNED_MODULE = "googletest 1.18.0"

CC_DEPCHECK_MANIFEST = "tools/depcheck/testdata/cc/ok_used/cc_deps.toml"
CC_DEPCHECK_LOCK = "tools/depcheck/testdata/cc/ok_used/cc_lock.json"

CC_FIXTURE_HELLO = "//cc/tests/fixtures/hello:hello_test"
CC_FIXTURE_GTEST = "//cc/tests/fixtures/googletest:greeter_test"

CC_REJECTED = "system packages rejected: no host apt/brew, no /usr paths, no local config; every http_archive carries sha256/integrity"
