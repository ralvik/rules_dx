"""Deterministic synthetic adapter registry (WP2 fixtures).

Contract: `docs/quality/tool-integrations.md`, `docs/quality/quality-sources.md`, `docs/decisions/0003-action-granularity.md`.
"""

# Versioned registry schema for the adapter taxonomy (issue #321).
# Consumers query via `real_supported_classes`, `is_known_adapter_tool`,
# `is_classified`, and `adapter_registry_schema_error` instead of
# duplicating the maps, so adding a language/tool edits this one data
# registry plus parity/compat, never a parallel allowlist.
ADAPTER_REGISTRY_SCHEMA_VERSION = 1

# Synthetic tool ID to capability to supported semantic file classes.
# Mirrors the `//quality:fixture_policy` shape: `lint-a` is selected by two
# families (proves cross-family union into one stage), `lint-b` and `fmt-a`
# are rust-only (prove exact subsets and python exclusion from format).
SYNTHETIC_ADAPTERS = {
    "fmt-a": {"format": ["rust"]},
    "lint-a": {"lint": ["python", "rust"]},
    "lint-b": {"lint": ["rust"]},
}

# WP2 fixture class-to-family assignment. The frozen taxonomy lives in
# REAL_CLASS_TO_FAMILY below; this minimal map only authorizes the fixture
# families above.
SYNTHETIC_CLASS_TO_FAMILY = {
    "python": "python",
    "rust": "rust",
}

def adapter_supported_classes(tool_id, capability):
    """Returns the sorted supported classes for one synthetic tool/capability.

    Fails on an unknown tool ID (matches `applicability.selected_adapters`
    failing "during configuration or analysis"). An unsupported capability
    returns [], so the stage is omitted and no empty action is created.
    """
    if tool_id not in SYNTHETIC_ADAPTERS:
        fail("adapters: unknown tool '" + tool_id +
             "': not in the synthetic adapter registry")
    return sorted(SYNTHETIC_ADAPTERS[tool_id].get(capability, []))

# Real adapters: stable tool IDs users select in policy families.
# Stages order by sorted tool ID; rustc is upstream-delegated and
# tsc is target-coupled. See `docs/quality/tool-integrations.md`.
REAL_ADAPTERS = {
    "biome": {
        "format": ["javascript", "json", "jsx", "typescript", "tsx"],
        "lint": ["javascript", "json", "jsx", "typescript", "tsx"],
    },
    "buildifier": {"format": ["starlark"], "lint": ["starlark"]},
    "clippy": {"lint": ["rust"]},
    "eslint": {"lint": ["javascript", "jsx"]},
    "flake8": {"lint": ["python", "python_stub"]},
    "markdown_check": {"lint": ["markdown"]},
    "prettier": {"format": ["javascript", "json", "jsx", "typescript", "tsx"]},
    "pydoclint": {"lint": ["python", "python_stub"]},
    "pylint": {"lint": ["python", "python_stub"]},
    "ruff": {"format": ["python", "python_stub"], "lint": ["python", "python_stub"]},
    "rustc": {"typecheck": ["rust"]},
    "rustfmt": {"format": ["rust"]},
    "taplo": {"format": ["toml"], "lint": ["toml"]},
    "tsc": {"typecheck": ["typescript", "tsx"]},
    "ty": {"typecheck": ["python", "python_stub"]},
    "vale": {"lint": ["markdown"]},
}

# Frozen taxonomy: one owning family per class (single-sourced here).
# Grouping and deferred adapters live in `docs/quality/quality-sources.md`
# and the parity gate; history lives in git log.
REAL_CLASS_TO_FAMILY = {
    "astro": "astro",
    "c": "cc",
    "cpp": "cc",
    "csharp": "csharp",
    "css": "css",
    "cuda": "cuda",
    "cue": "cue",
    "fsharp": "fsharp",
    "gherkin": "gherkin",
    "go": "go",
    "go_module": "go_module",
    "graphql": "graphql",
    "html": "html",
    "html_template": "html_template",
    "java": "java",
    "javascript": "javascript",
    "json": "json",
    "json5": "json",
    "jsonc": "json",
    "jsx": "javascript",
    "kotlin": "kotlin",
    "less": "css",
    "markdown": "markdown",
    "mdx": "mdx",
    "pkl": "pkl",
    "protobuf": "protobuf",
    "powershell": "powershell",
    "python": "python",
    "python_stub": "python",
    "qml": "qml",
    "ruby": "ruby",
    "rust": "rust",
    "scala": "scala",
    "scss": "css",
    "shell": "shell",
    "sql": "sql",
    "starlark": "starlark",
    "svelte": "svelte",
    "terraform": "terraform",
    "text": "text",
    "toml": "toml",
    "typescript": "typescript",
    "tsx": "typescript",
    "vue": "vue",
    "jsonnet": "jsonnet",
    "xml": "xml",
    "yaml": "yaml",
}

def real_supported_classes(tool_id, capability):
    """Returns the sorted supported classes for one real tool/capability.

    Fails on an unknown tool ID during configuration or analysis, matching
    the tool-integrations contract. An unsupported capability returns [],
    so the stage is omitted and no empty action is created.
    """
    if tool_id not in REAL_ADAPTERS:
        fail("adapters: unknown tool '" + tool_id +
             "': not in the real adapter registry")
    return sorted(REAL_ADAPTERS[tool_id].get(capability, []))

def is_known_adapter_tool(tool_id):
    """Reports whether a tool ID is in the versioned adapter registry."""
    return tool_id in REAL_ADAPTERS

def is_classified(class_id):
    """Reports whether a class is in the versioned class-to-family map."""
    return class_id in REAL_CLASS_TO_FAMILY

def registry_families():
    """Returns the sorted unique owning families in the registry."""
    seen = {}
    for class_id in REAL_CLASS_TO_FAMILY:
        seen[REAL_CLASS_TO_FAMILY[class_id]] = True
    return sorted(seen.keys())

def registry_tools():
    """Returns the sorted known real adapter tool IDs."""
    return sorted(REAL_ADAPTERS.keys())

def _is_canonical_token(text):
    if text == "":
        return False
    for c in text.elems():
        if c not in "abcdefghijklmnopqrstuvwxyz0123456789_":
            return False
    return True

def adapter_registry_schema_error():
    """Validates the versioned adapter-registry schema (issue #321).

    Checks data shape without pinning exact contents, so adding a
    language/tool edits the registry data only: version is v1, every
    class and family spelling is canonical, every adapter capability
    names a known capability with canonical classes, and every
    adapter-backed class is classified."""
    if ADAPTER_REGISTRY_SCHEMA_VERSION != 1:
        return "adapter registry: unsupported schema v" + str(ADAPTER_REGISTRY_SCHEMA_VERSION) + " (want v1)"
    for class_id in REAL_CLASS_TO_FAMILY:
        if not _is_canonical_token(class_id):
            return "adapter registry: non-canonical class '" + str(class_id) + "'"
        family = REAL_CLASS_TO_FAMILY[class_id]
        if not _is_canonical_token(family):
            return "adapter registry: non-canonical family '" + str(family) + "' for class '" + class_id + "'"
    for tool_id in REAL_ADAPTERS:
        if not _is_canonical_token(tool_id):
            return "adapter registry: non-canonical tool '" + str(tool_id) + "'"
        for capability in REAL_ADAPTERS[tool_id]:
            if capability not in ["audit", "format", "lint", "typecheck"]:
                return "adapter registry: unknown capability '" + capability + "' for tool '" + tool_id + "'"
            for class_id in REAL_ADAPTERS[tool_id][capability]:
                if not _is_canonical_token(class_id):
                    return "adapter registry: non-canonical class '" + str(class_id) + "' for tool '" + tool_id + "'"
                if class_id not in REAL_CLASS_TO_FAMILY:
                    return "adapter registry: tool '" + tool_id + "' names unclassified class '" + class_id + "'"
    return ""
