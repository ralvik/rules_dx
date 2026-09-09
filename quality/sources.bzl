"""Quality source-ownership boundary (M03 freeze for O15, ADR 0013).

`QualitySourcesInfo` tells quality aspects which directly owned repository
source artifacts may be linted, typechecked, formatted, or audited. It
carries no capabilities, tools, configs, dependencies, generated context,
transitive sources, or execution metadata.

Contract: `docs/quality/quality-sources.md`. The provider concept and the
`direct_sources` shape are accepted there; the constructor, load label
(`//quality:sources.bzl`), field representation, and registry below are
frozen here. Class IDs are public compatibility surface: renaming,
removing, merging, or semantically narrowing an ID is breaking; adding a
class or broadening one requires adapter and policy compatibility tests.

Validation split (per O15 direction): construction validates field shape
and known IDs only. Direct ownership, admissibility, and single-class
membership are the consuming aspect's job (M04+), not this file's. The
class-to-policy-family assignment and admissibility mappings stay pending
the registry review; they are not frozen here.
"""

QualitySourcesInfo = provider(
    doc = "Directly owned repository sources keyed by semantic file class.",
    fields = {
        "direct_sources": (
            "Dict[str, depset[File]]: semantic file-class ID to directly " +
            "owned source artifacts. Empty depsets are omitted, never retained."
        ),
    },
)

# Candidate canonical semantic file-class IDs from
# docs/quality/quality-sources.md. Provisional pending O15; adding a class
# or broadening one requires adapter and policy compatibility tests.
KNOWN_SEMANTIC_FILE_CLASSES = [
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

# Semantic file class for Rust sources (M02 proves this one).
RUST = "rust"

def check_direct_sources(direct_sources, what):
    """Validates provider-construction shape and known IDs.

    Fails analysis on: non-dict map, unknown class ID, non-depset value,
    or non-File member. Ownership, admissibility, and single-class
    membership are validated by the consuming aspect, not here.
    """
    if type(direct_sources) != "dict":
        fail("QualitySourcesInfo (" + what + "): direct_sources must be a " +
             "dict of class ID to depset, got " + type(direct_sources))
    for class_id in direct_sources.keys():
        if class_id not in KNOWN_SEMANTIC_FILE_CLASSES:
            fail("QualitySourcesInfo (" + what + "): unknown semantic " +
                 "file class '" + class_id + "'")
        sources = direct_sources[class_id]
        if type(sources) != "depset":
            fail("QualitySourcesInfo (" + what + "): class '" + class_id +
                 "' must map to a depset of Files, got " + type(sources))
        for f in sources.to_list():
            if type(f) != "File":
                fail("QualitySourcesInfo (" + what + "): class '" + class_id +
                     "' holds a non-File member: " + type(f))
