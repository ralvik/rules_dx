load("//libs/starlark:defs.bzl", "expect_contains", "expect_equal", "expect_false", "expect_match", "expect_true", "starlark_test")
load("//libs/starlark/tests/fixtures/starlark_futures:toolchain_subjects.bzl", "admitted_platforms", "admitted_toolchains", "is_supported_platform", "resolve_toolchain", "toolchain_fingerprint_like", "toolchain_report", "toolchain_subject_fields")

def toolchain_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("resolves linux to gcc", resolve_toolchain("linux_x86_64", {"linux_x86_64": "gcc", "macos_arm64": "clang"}), "gcc"),
            expect_equal("resolves macos to clang", resolve_toolchain("macos_arm64", {"linux_x86_64": "gcc", "macos_arm64": "clang"}), "clang"),
            expect_true("linux_x86_64 is supported", is_supported_platform("linux_x86_64")),
            expect_false("windows is not supported", is_supported_platform("windows_x86_64")),
            expect_contains("report mentions toolchain", toolchain_report("linux_x86_64", "gcc"), "gcc"),
            expect_contains("report mentions platform", toolchain_report("linux_x86_64", "gcc"), "linux_x86_64"),
            expect_contains("admitted platforms contain linux", admitted_platforms(), "linux_x86_64"),
            expect_contains("admitted toolchains contain gcc", admitted_toolchains(), "gcc"),
            expect_contains("resolve error mentions missing platform", resolve_toolchain("windows_x86_64", {"linux_x86_64": "gcc"}), "windows_x86_64"),
            expect_contains("subject fields contain toolchain key", toolchain_subject_fields("linux_x86_64", "gcc"), "toolchain"),
            expect_match("fingerprint mentions toolchain without pinning full JSON", toolchain_fingerprint_like("gcc"), "gcc"),
            expect_match("rendered list mentions platform substring", ["linux_x86_64-gcc"], "linux_x86_64"),
        ],
    )
