"""GoogleTest v1.18.0 plus C++17 floor version pins (issue #479).

Contract: `docs/product/support-matrix.md#provisional-default-test-runners`,
`docs/generation/README.md#language-mapping-qualification`.

Pinned line is GoogleTest v1.18.0 (Bazel Central Registry module
`googletest` 1.18.0, verified against Bazel 9.2.0 on the seed host). The
1.18.x branch requires C++17 or newer per the upstream v1.18.0 release
notes; the qualified toolchain floor is `-std=c++17` on the seed host
(gcc 13 defaults to gnu++17, the fixture pins the floor explicitly so the
proof never relies on the compiler default). Living at head (unpinned or
floating runner) is rejected per the dx pin policy.
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
