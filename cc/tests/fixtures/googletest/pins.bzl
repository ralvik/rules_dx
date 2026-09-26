"""GoogleTest v1.18.0 plus C++17 floor version pins."""

GTEST_VERSION = "1.18.0"

GTEST_CXX_FLOOR = "17"
GTEST_COPTS = ["-std=c++17"]

GTEST_MAIN_LABEL = "@googletest//:gtest_main"
GTEST_LABEL = "@googletest//:gtest"

GTEST_REJECTED = "unpinned runner rejected: no floating version or head"

GTEST_FIXTURE_TEST = "cc/tests/fixtures/googletest:greeter_test"
