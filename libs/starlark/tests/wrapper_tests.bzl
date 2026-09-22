"""Unit tests proving optional-provider forwarding warns instead of silently skipping (issue #943).
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load("//libs/starlark:wrapper.bzl", "dx_missing_optional_names", "dx_optional_forward_warning")

def wrapper_optional_forward_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "empty request forwards nothing without warning",
                dx_missing_optional_names([], ["CcInfo"]),
                [],
            ),
            expect_equal(
                "empty request warns nothing",
                dx_optional_forward_warning("go_*", "//go:bin_upstream", [], []),
                None,
            ),
            expect_equal(
                "all present misses nothing",
                dx_missing_optional_names(["GoArchive"], ["GoArchive"]),
                [],
            ),
            expect_equal(
                "all present warns nothing",
                dx_optional_forward_warning("go_*", "//go:bin_upstream", ["GoArchive"], []),
                None,
            ),
            expect_equal(
                "partial skip reports only the missing provider",
                dx_missing_optional_names(
                    ["InstrumentedFilesInfo", "OutputGroupInfo"],
                    ["OutputGroupInfo"],
                ),
                ["InstrumentedFilesInfo"],
            ),
            expect_equal(
                "partial warning names the missing provider and the forward count",
                dx_optional_forward_warning(
                    "javascript_*",
                    "//js:bin_upstream",
                    ["InstrumentedFilesInfo", "OutputGroupInfo"],
                    ["InstrumentedFilesInfo"],
                ),
                "javascript_*: upstream //js:bin_upstream omits optional provider(s) InstrumentedFilesInfo (forwarded 1 of 2)",
            ),
            expect_equal(
                "total skip reports every requested provider",
                dx_missing_optional_names(["CcInfo"], []),
                ["CcInfo"],
            ),
            expect_equal(
                "empty forward warns and marks the empty case expected",
                dx_optional_forward_warning("go_*", "//go:bin_upstream", ["GoArchive"], ["GoArchive"]),
                "go_*: upstream //go:bin_upstream omits optional provider(s) GoArchive (forwarded 0 of 1); empty forward is expected when upstream omits the surface",
            ),
        ],
    )
