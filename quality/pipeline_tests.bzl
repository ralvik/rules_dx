"""Unit tests for pipeline construction (M03 WP2a).

Pins the synthetic cross-family union, exact source subsets, stable tool
order, and no-empty-stage rules. Analysis evidence (exact direct-source
subsets, no generic fallback, no empty actions) lands in WP2c; these unit
checks prove the pure shapes that evidence rests on.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":adapters.bzl", "SYNTHETIC_ADAPTERS", "SYNTHETIC_CLASS_TO_FAMILY", "adapter_supported_classes")
load(":pipeline.bzl", "authorize_classes", "pipeline_stages", "resolve_pipeline", "stage_sources")

_LINT_SELECTIONS = {
    "python": ["lint-a"],
    "rust": ["lint-a", "lint-b"],
}

_FORMAT_SELECTIONS = {
    "python": [],
    "rust": ["fmt-a"],
}

_EMPTY_SELECTIONS = {
    "python": [],
    "rust": [],
}

_DIRECT_SOURCES = {
    "python": ["src/main.py"],
    "rust": ["src/lib.rs", "src/main.rs"],
}

def pipeline_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "adapter_supported_classes returns sorted lint support",
                adapter_supported_classes("lint-a", "lint"),
                ["python", "rust"],
            ),
            expect_equal(
                "adapter_supported_classes returns [] for unsupported capability",
                adapter_supported_classes("lint-a", "format"),
                [],
            ),
            expect_equal(
                "adapter_supported_classes returns rust-only format support",
                adapter_supported_classes("fmt-a", "format"),
                ["rust"],
            ),
            expect_equal(
                "authorize_classes unions cross-family selection into one tool",
                authorize_classes(_LINT_SELECTIONS, SYNTHETIC_CLASS_TO_FAMILY),
                {"lint-a": ["python", "rust"], "lint-b": ["rust"]},
            ),
            expect_equal(
                "authorize_classes keeps format authorized to rust only",
                authorize_classes(_FORMAT_SELECTIONS, SYNTHETIC_CLASS_TO_FAMILY),
                {"fmt-a": ["rust"]},
            ),
            expect_equal(
                "pipeline_stages builds lint union plus rust-only stage in tool order",
                pipeline_stages(
                    ["rust", "python"],
                    "lint",
                    _LINT_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
                [
                    {"classes": ["python", "rust"], "tool": "lint-a"},
                    {"classes": ["rust"], "tool": "lint-b"},
                ],
            ),
            expect_equal(
                "pipeline_stages is independent of target class order",
                pipeline_stages(
                    ["python", "rust"],
                    "lint",
                    _LINT_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
                pipeline_stages(
                    ["rust", "python"],
                    "lint",
                    _LINT_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
            ),
            expect_equal(
                "pipeline_stages restricts format to rust sources",
                pipeline_stages(
                    ["rust", "python"],
                    "format",
                    _FORMAT_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
                [{"classes": ["rust"], "tool": "fmt-a"}],
            ),
            expect_equal(
                "pipeline_stages omits adapters without effective sources",
                pipeline_stages(
                    ["markdown"],
                    "lint",
                    _LINT_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
                [],
            ),
            expect_equal(
                "pipeline_stages is empty for an unselected capability",
                pipeline_stages(
                    ["rust", "python"],
                    "typecheck",
                    _EMPTY_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
                [],
            ),
            expect_equal(
                "stage_sources unions and sorts exact subsets",
                stage_sources(_DIRECT_SOURCES, ["rust", "python"]),
                ["src/lib.rs", "src/main.py", "src/main.rs"],
            ),
            expect_equal(
                "stage_sources ignores classes without direct sources",
                stage_sources({"rust": ["src/lib.rs"]}, ["rust", "python"]),
                ["src/lib.rs"],
            ),
            expect_equal(
                "resolve_pipeline carries exact per-stage source subsets",
                resolve_pipeline(
                    ["rust", "python"],
                    _DIRECT_SOURCES,
                    "lint",
                    _LINT_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
                [
                    {
                        "classes": ["python", "rust"],
                        "sources": ["src/lib.rs", "src/main.py", "src/main.rs"],
                        "tool": "lint-a",
                    },
                    {
                        "classes": ["rust"],
                        "sources": ["src/lib.rs", "src/main.rs"],
                        "tool": "lint-b",
                    },
                ],
            ),
            expect_equal(
                "resolve_pipeline omits stages with no files present",
                resolve_pipeline(
                    ["rust", "python"],
                    {"rust": ["src/lib.rs"]},
                    "format",
                    _FORMAT_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
                [
                    {
                        "classes": ["rust"],
                        "sources": ["src/lib.rs"],
                        "tool": "fmt-a",
                    },
                ],
            ),
            expect_equal(
                "resolve_pipeline is empty without effective sources",
                resolve_pipeline(
                    ["markdown"],
                    {"markdown": ["doc.md"]},
                    "lint",
                    _LINT_SELECTIONS,
                    SYNTHETIC_CLASS_TO_FAMILY,
                    SYNTHETIC_ADAPTERS,
                ),
                [],
            ),
        ],
    )
