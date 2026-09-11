"""Unit tests for real-adapter pipeline construction (M04 WP2, M12 WP3).

Pins the initial-adapter capability manifests (O20) through the same pure
pipeline formula the synthetic fixtures use: exact class-to-tool mapping,
sorted tool order across mixed-class targets, exact per-stage source
subsets, and omission of unsupported classes/capabilities (no empty
actions). Cross-toolchain quality evidence (exact-input, no-config, edit,
cache, empty-PATH) lands in later WP2 commits; these checks prove the
pure shapes that evidence rests on. M12 WP3 adds the rustc typecheck
stage over the rust class.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":adapters.bzl", "REAL_ADAPTERS", "REAL_CLASS_TO_FAMILY", "real_supported_classes")
load(":pipeline.bzl", "authorize_classes", "pipeline_stages", "resolve_pipeline")

_LINT_SELECTIONS = {
    "markdown": ["markdown_check", "vale"],
    "rust": ["clippy"],
    "starlark": ["buildifier"],
    "toml": ["taplo"],
}

_FORMAT_SELECTIONS = {
    "markdown": [],
    "rust": ["rustfmt"],
    "starlark": ["buildifier"],
    "toml": ["taplo"],
}

_TYPECHECK_SELECTIONS = {
    "markdown": [],
    "rust": ["rustc"],
    "starlark": [],
    "toml": [],
}

_DIRECT_SOURCES = {
    "markdown": ["doc/guide.md"],
    "rust": ["src/lib.rs", "src/main.rs"],
    "starlark": ["BUILD.bazel"],
    "toml": ["Cargo.toml"],
}

def real_pipeline_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "real_supported_classes returns buildifier lint support",
                real_supported_classes("buildifier", "lint"),
                ["starlark"],
            ),
            expect_equal(
                "real_supported_classes returns buildifier format support",
                real_supported_classes("buildifier", "format"),
                ["starlark"],
            ),
            expect_equal(
                "real_supported_classes returns clippy lint support",
                real_supported_classes("clippy", "lint"),
                ["rust"],
            ),
            expect_equal(
                "real_supported_classes returns [] for unsupported clippy format",
                real_supported_classes("clippy", "format"),
                [],
            ),
            expect_equal(
                "real_supported_classes returns rustfmt format support",
                real_supported_classes("rustfmt", "format"),
                ["rust"],
            ),
            expect_equal(
                "real_supported_classes returns [] for unsupported rustfmt lint",
                real_supported_classes("rustfmt", "lint"),
                [],
            ),
            expect_equal(
                "real_supported_classes returns taplo lint and format support",
                [
                    real_supported_classes("taplo", "lint"),
                    real_supported_classes("taplo", "format"),
                ],
                [["toml"], ["toml"]],
            ),
            expect_equal(
                "real_supported_classes returns markdown_check lint support only",
                [
                    real_supported_classes("markdown_check", "lint"),
                    real_supported_classes("markdown_check", "format"),
                ],
                [["markdown"], []],
            ),
            expect_equal(
                "real_supported_classes returns vale lint support only",
                [
                    real_supported_classes("vale", "lint"),
                    real_supported_classes("vale", "format"),
                ],
                [["markdown"], []],
            ),
            expect_equal(
                "real_supported_classes returns rustc typecheck support only",
                [
                    real_supported_classes("rustc", "typecheck"),
                    real_supported_classes("rustc", "lint"),
                    real_supported_classes("rustc", "format"),
                ],
                [["rust"], [], []],
            ),
            expect_equal(
                "authorize_classes maps each real tool to its own class",
                authorize_classes(_LINT_SELECTIONS, REAL_CLASS_TO_FAMILY),
                {
                    "buildifier": ["starlark"],
                    "clippy": ["rust"],
                    "markdown_check": ["markdown"],
                    "taplo": ["toml"],
                    "vale": ["markdown"],
                },
            ),
            expect_equal(
                "pipeline_stages builds one lint stage per tool in tool order",
                pipeline_stages(
                    ["toml", "rust", "markdown", "starlark"],
                    "lint",
                    _LINT_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [
                    {"classes": ["starlark"], "tool": "buildifier"},
                    {"classes": ["rust"], "tool": "clippy"},
                    {"classes": ["markdown"], "tool": "markdown_check"},
                    {"classes": ["toml"], "tool": "taplo"},
                    {"classes": ["markdown"], "tool": "vale"},
                ],
            ),
            expect_equal(
                "pipeline_stages builds format stages in tool order",
                pipeline_stages(
                    ["toml", "starlark", "rust"],
                    "format",
                    _FORMAT_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [
                    {"classes": ["starlark"], "tool": "buildifier"},
                    {"classes": ["rust"], "tool": "rustfmt"},
                    {"classes": ["toml"], "tool": "taplo"},
                ],
            ),
            expect_equal(
                "pipeline_stages builds one rustc typecheck stage",
                pipeline_stages(
                    ["toml", "starlark", "rust"],
                    "typecheck",
                    _TYPECHECK_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [
                    {"classes": ["rust"], "tool": "rustc"},
                ],
            ),
            expect_equal(
                "pipeline_stages omits typecheck with no rust sources",
                pipeline_stages(
                    ["toml", "starlark"],
                    "typecheck",
                    _TYPECHECK_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [],
            ),
            expect_equal(
                "pipeline_stages omits classes no real adapter supports",
                pipeline_stages(
                    ["python"],
                    "lint",
                    _LINT_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [],
            ),
            expect_equal(
                "pipeline_stages omits markdown format (markdown lint tools are lint-only)",
                pipeline_stages(
                    ["markdown"],
                    "format",
                    _FORMAT_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [],
            ),
            expect_equal(
                "resolve_pipeline carries exact per-stage source subsets",
                resolve_pipeline(
                    ["toml", "rust", "markdown", "starlark"],
                    _DIRECT_SOURCES,
                    "lint",
                    _LINT_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [
                    {
                        "classes": ["starlark"],
                        "sources": ["BUILD.bazel"],
                        "tool": "buildifier",
                    },
                    {
                        "classes": ["rust"],
                        "sources": ["src/lib.rs", "src/main.rs"],
                        "tool": "clippy",
                    },
                    {
                        "classes": ["markdown"],
                        "sources": ["doc/guide.md"],
                        "tool": "markdown_check",
                    },
                    {
                        "classes": ["toml"],
                        "sources": ["Cargo.toml"],
                        "tool": "taplo",
                    },
                    {
                        "classes": ["markdown"],
                        "sources": ["doc/guide.md"],
                        "tool": "vale",
                    },
                ],
            ),
            expect_equal(
                "resolve_pipeline omits real stages with no files present",
                resolve_pipeline(
                    ["toml", "starlark", "rust"],
                    {"rust": ["src/lib.rs"]},
                    "format",
                    _FORMAT_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [
                    {
                        "classes": ["rust"],
                        "sources": ["src/lib.rs"],
                        "tool": "rustfmt",
                    },
                ],
            ),
            expect_equal(
                "resolve_pipeline carries the exact typecheck source subset",
                resolve_pipeline(
                    ["toml", "starlark", "rust"],
                    _DIRECT_SOURCES,
                    "typecheck",
                    _TYPECHECK_SELECTIONS,
                    REAL_CLASS_TO_FAMILY,
                    REAL_ADAPTERS,
                ),
                [
                    {
                        "classes": ["rust"],
                        "sources": ["src/lib.rs", "src/main.rs"],
                        "tool": "rustc",
                    },
                ],
            ),
        ],
    )
