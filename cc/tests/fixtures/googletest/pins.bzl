"""GoogleTest v1.18.0 plus C++17 floor version pins.

Contract: `docs/product/support-matrix.md#provisional-default-test-runners`,
`docs/generation/README.md#language-mapping-qualification`.
"""

# Pinned line: GoogleTest 1.18.0 (BCR module `googletest` 1.18.0).
GTEST_VERSION = "1.18.0"

# C++ floor mapping: 17 selects the 1.18.x line; below the floor is rejected.
GTEST_CXX_FLOOR = "17"
GTEST_COPTS = ["-std=c++17"]

# Runner mapping: plain `cc_test` over `@googletest//:gtest_main` with
# `TEST()` plus `EXPECT_*` sources; the library under test stays its
# ordinary owner via `deps`.
GTEST_MAIN_LABEL = "@googletest//:gtest_main"
GTEST_LABEL = "@googletest//:gtest"

# Rejected: unpinned runner (floating version, living at head, implicit).
GTEST_REJECTED = "unpinned runner rejected: no floating version or head"

GTEST_FIXTURE_TEST = "cc/tests/fixtures/googletest:greeter_test"
