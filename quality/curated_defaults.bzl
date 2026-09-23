"""Frozen v1 curated quality defaults.

Contract: `docs/tools/tool-baseline.md#curated-differences`, `docs/tools/tool-baseline.md#default-lifecycle-direction`, `docs/quality/quality-testing.md`.
"""

# Versioned curated-defaults schema. Consumers query via
# `curated_families`, `curated_tools`, and `curated_schema_error` instead
# of duplicating the manifest, so adding a curated family edits this one
# data manifest plus compat evidence, never a parallel allowlist.
CURATED_SCHEMA_VERSION = 1

# Family -> capability -> curated default tool IDs, in stable pipeline
# order. Families absent from this map have no curated defaults yet
# (their classes are PARITY_DEFERRED in quality/parity_tests.bzl).
CURATED_DEFAULTS = {
    "java": {
        "audit": [],
        "format": ["google_java_format"],
        "lint": ["checkstyle", "pmd", "spotbugs"],
        "typecheck": [],
    },
    "javascript": {
        "audit": [],
        "format": ["biome"],
        "lint": ["biome"],
        "typecheck": [],
    },
    "json": {
        "audit": [],
        "format": ["prettier"],
        "lint": ["biome"],
        "typecheck": [],
    },
    "kotlin": {
        "audit": [],
        "format": ["ktfmt"],
        "lint": ["ktlint"],
        "typecheck": [],
    },
    "markdown": {
        "audit": [],
        "format": [],
        "lint": ["markdown_check", "vale"],
        "typecheck": [],
    },
    "python": {
        "audit": [],
        "format": ["ruff"],
        "lint": ["pydoclint", "ruff"],
        "typecheck": ["ty"],
    },
    "rust": {
        "audit": [],
        "format": ["rustfmt"],
        "lint": ["clippy"],
        "typecheck": ["rustc"],
    },
    "starlark": {
        "audit": [],
        "format": ["buildifier"],
        "lint": ["buildifier"],
        "typecheck": [],
    },
    "toml": {
        "audit": [],
        "format": ["taplo"],
        "lint": ["taplo"],
        "typecheck": [],
    },
    "typescript": {
        "audit": [],
        "format": ["biome"],
        "lint": ["biome"],
        "typecheck": ["tsc"],
    },
}

# Frozen default formatter set per family: changing any entry requires a
# major release (never a minor). The harness greps this file for the
# exact lines below, so keep the `FORMAT_FROZEN[<family>] = [...]`
# shape stable.
FORMAT_FROZEN = {
    "java": ["google_java_format"],
    "javascript": ["biome"],
    "json": ["prettier"],
    "kotlin": ["ktfmt"],
    "markdown": [],
    "python": ["ruff"],
    "rust": ["rustfmt"],
    "starlark": ["buildifier"],
    "toml": ["taplo"],
    "typescript": ["biome"],
}

def curated_families():
    """Returns the sorted curated families in the versioned manifest."""
    return sorted(CURATED_DEFAULTS.keys())

def curated_tools():
    """Returns the sorted unique curated tool IDs across families.

    Returns:
      Sorted unique tool IDs across all curated families.
    """
    seen = {}
    for family in CURATED_DEFAULTS:
        for capability in CURATED_DEFAULTS[family]:
            for tool in CURATED_DEFAULTS[family][capability]:
                seen[tool] = True
    return sorted(seen.keys())

def _is_canonical_token(text):
    if text == "":
        return False
    for c in text.elems():
        if c not in "abcdefghijklmnopqrstuvwxyz0123456789_":
            return False
    return True

def curated_schema_error():
    """Validates the versioned curated-defaults schema (See: `docs/tools/tool-baseline.md#curated-differences`, issue #321).

    Checks data shape without pinning exact contents, so adding a curated
    family edits the manifest data only: version is v1, every family and
    tool spelling is canonical, every family carries exactly the
    audit/format/lint/typecheck capabilities, and the frozen formatter
    set matches the curated format selection.

    Returns:
      Empty string when the schema is valid; otherwise a diagnostic.
    """
    if CURATED_SCHEMA_VERSION != 1:
        return "curated defaults: unsupported schema v" + str(CURATED_SCHEMA_VERSION) + " (want v1)"
    for family in CURATED_DEFAULTS:
        if not _is_canonical_token(family):
            return "curated defaults: non-canonical family '" + str(family) + "'"
        entry = CURATED_DEFAULTS[family]
        if sorted(entry.keys()) != ["audit", "format", "lint", "typecheck"]:
            return "curated defaults: family '" + family + "' must carry exactly audit/format/lint/typecheck"
        for capability in entry:
            for tool in entry[capability]:
                if not _is_canonical_token(tool):
                    return "curated defaults: non-canonical tool '" + str(tool) + "' for family '" + family + "'"
        if family in FORMAT_FROZEN:
            if FORMAT_FROZEN[family] != entry["format"]:
                return "curated defaults: FORMAT_FROZEN drift for family '" + family + "' (formatter-set changes need a major release)"
        else:
            return "curated defaults: family '" + family + "' missing from FORMAT_FROZEN"
    for family in FORMAT_FROZEN:
        if family not in CURATED_DEFAULTS:
            return "curated defaults: FORMAT_FROZEN family '" + family + "' has no curated entry"
    return ""
