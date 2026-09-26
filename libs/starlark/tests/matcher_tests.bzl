load("//libs/starlark:defs.bzl", "expect_contains", "expect_false", "expect_match", "expect_true", "starlark_test")
load("//libs/starlark/tests/fixtures/starlark_futures:matchers.bzl", "admitted_pairs", "fingerprint_like", "greet_report", "is_even", "pair_error", "subject_fields")

def matcher_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_true("even 42", is_even(42)),
            expect_false("odd 3 is not even", is_even(3)),
            expect_true("admitted pair has empty error", pair_error("protobuf", "rust") == ""),
            expect_contains("greeting mentions name", greet_report("world"), "world"),
            expect_contains("admitted list contains protobuf/rust", admitted_pairs(), "protobuf/rust"),
            expect_contains("pair error mentions pair", pair_error("graphql", "rust"), "graphql/rust"),
            expect_contains("subject fields contain sum key", subject_fields(), "sum"),
            expect_match("fingerprint mentions producer without pinning full JSON", fingerprint_like("//gen:alpha"), "//gen:alpha"),
            expect_match("rendered list mentions element substring", ["alpha-beta"], "alpha"),
        ],
    )
