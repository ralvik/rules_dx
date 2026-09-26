"""Unit tests proving the action-subjects use case."""

load("//libs/starlark:defs.bzl", "expect_contains", "expect_equal", "expect_false", "expect_match", "expect_true", "starlark_test")
load("//libs/starlark/tests/fixtures/starlark_futures:action_subjects.bzl", "action_fingerprint_like", "action_report", "action_subject_fields", "admitted_action_outputs", "admitted_actions", "is_supported_action", "resolve_action")

def action_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("resolves StarlarkAction", resolve_action("StarlarkAction", {"FileWrite": ["report.txt"], "StarlarkAction": ["out.txt"]}), ["out.txt"]),
            expect_equal("resolves FileWrite", resolve_action("FileWrite", {"FileWrite": ["report.txt"], "StarlarkAction": ["out.txt"]}), ["report.txt"]),
            expect_true("StarlarkAction is supported", is_supported_action("StarlarkAction")),
            expect_false("CppCompile is not supported", is_supported_action("CppCompile")),
            expect_contains("report mentions mnemonic", action_report("StarlarkAction", ["out.txt"]), "StarlarkAction"),
            expect_contains("report mentions output", action_report("StarlarkAction", ["out.txt"]), "out.txt"),
            expect_contains("admitted actions contain StarlarkAction", admitted_actions(), "StarlarkAction"),
            expect_contains("admitted outputs contain out", admitted_action_outputs(), "out.txt"),
            expect_contains("resolve error mentions missing mnemonic", resolve_action("CppCompile", {"StarlarkAction": ["out.txt"]}), "CppCompile"),
            expect_contains("subject fields contain mnemonic key", action_subject_fields("StarlarkAction", ["out.txt"]), "mnemonic"),
            expect_match("fingerprint mentions mnemonic without pinning full JSON", action_fingerprint_like("StarlarkAction"), "StarlarkAction"),
            expect_match("rendered list mentions output substring", ["out.txt-report.txt"], "out.txt"),
        ],
    )
