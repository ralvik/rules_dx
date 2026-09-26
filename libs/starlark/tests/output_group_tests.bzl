"""Unit tests proving the output-group-subjects use case."""

load("//libs/starlark:defs.bzl", "expect_contains", "expect_equal", "expect_false", "expect_match", "expect_true", "starlark_test")
load("//libs/starlark/tests/fixtures/starlark_futures:output_group_subjects.bzl", "admitted_output_files", "admitted_output_groups", "is_supported_output_group", "output_group_fingerprint_like", "output_group_report", "output_group_subject_fields", "resolve_output_group")

def output_group_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("resolves docs group", resolve_output_group("docs", {"artifacts": ["bundle.zip"], "docs": ["guide.md"]}), ["guide.md"]),
            expect_equal("resolves artifacts group", resolve_output_group("artifacts", {"artifacts": ["bundle.zip"], "docs": ["guide.md"]}), ["bundle.zip"]),
            expect_true("docs is supported", is_supported_output_group("docs")),
            expect_false("logs is not supported", is_supported_output_group("logs")),
            expect_contains("report mentions group", output_group_report("docs", ["guide.md"]), "docs"),
            expect_contains("report mentions file", output_group_report("docs", ["guide.md"]), "guide.md"),
            expect_contains("admitted groups contain docs", admitted_output_groups(), "docs"),
            expect_contains("admitted files contain guide", admitted_output_files(), "guide.md"),
            expect_contains("resolve error mentions missing group", resolve_output_group("logs", {"docs": ["guide.md"]}), "logs"),
            expect_contains("subject fields contain group key", output_group_subject_fields("docs", ["guide.md"]), "group"),
            expect_match("fingerprint mentions group without pinning full JSON", output_group_fingerprint_like("docs"), "docs"),
            expect_match("rendered list mentions file substring", ["guide.md-bundle.zip"], "guide.md"),
        ],
    )
