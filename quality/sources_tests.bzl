"""Versioned semantic-class registry tests (freeze).

Contract: `docs/quality/quality-sources.md`.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":sources.bzl", "RUST", "SOURCES_REGISTRY_SCHEMA_VERSION", "is_known_semantic_class", "sources_schema_error")

# Frozen core pin: every ID below must stay known. Additions append to the
# registry data without editing this list; removals/renames fail here plus
# adapter/parity compat.
FROZEN_SEMANTIC_FILE_CLASSES = [
    "text",
    "c",
    "cpp",
    "cuda",
    "csharp",
    "fsharp",
    "powershell",
    "css",
    "less",
    "scss",
    "javascript",
    "jsx",
    "typescript",
    "tsx",
    "vue",
    "svelte",
    "astro",
    "mdx",
    "graphql",
    "html",
    "html_template",
    "json",
    "json5",
    "jsonc",
    "markdown",
    "toml",
    "xml",
    "yaml",
    "java",
    "kotlin",
    "scala",
    "python",
    "python_stub",
    "cue",
    "gherkin",
    "go",
    "jsonnet",
    "pkl",
    "protobuf",
    "qml",
    "ruby",
    "rust",
    "shell",
    "sql",
    "starlark",
    "terraform",
    "go_module",
]

def sources_registry_tests(name):
    starlark_test(
        name = name,
        mode = "load",
        checks = [
            expect_equal(
                "sources schema version stays v1",
                SOURCES_REGISTRY_SCHEMA_VERSION,
                1,
            ),
            expect_equal(
                "sources registry schema validates",
                sources_schema_error(),
                "",
            ),
            expect_equal(
                "frozen core IDs stay known (additions need no allowlist edit)",
                [c for c in FROZEN_SEMANTIC_FILE_CLASSES if not is_known_semantic_class(c)],
                [],
            ),
            expect_equal("RUST class ID", RUST, "rust"),
        ],
    )
