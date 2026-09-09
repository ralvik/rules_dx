"""Load tests pinning the frozen semantic-class registry (M03 freeze for O15).

Class IDs are public compatibility surface: this test fails on any rename,
removal, merge, narrowing, reorder, or unreviewed addition. Adding a class
or broadening one requires adapter and policy compatibility tests first.
"""

load("//tools/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":sources.bzl", "KNOWN_SEMANTIC_FILE_CLASSES", "RUST")

# Frozen registry pin: mirrors KNOWN_SEMANTIC_FILE_CLASSES element for
# element, so any registry edit fails this test until the pin is reviewed.
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
                "registry matches the frozen pin",
                KNOWN_SEMANTIC_FILE_CLASSES,
                FROZEN_SEMANTIC_FILE_CLASSES,
            ),
            expect_equal("RUST class ID", RUST, "rust"),
        ],
    )
