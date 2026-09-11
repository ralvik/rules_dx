"""Deterministic synthetic adapter registry (M03 WP2 fixtures).

Provisional M03-only registry proving target/capability pipeline
construction without real tools. Tool IDs are synthetic; the real family
taxonomy and curated defaults belong to the tool baseline (M04+).

Contract: `docs/quality/tool-integrations.md`, `docs/quality/quality-sources.md`
(adapter applicability), `docs/decisions/0003-action-granularity.md`
(provisional stage order).

Stage order note (O19): WP2 uses lexical tool-ID order as the stable
ruleset order for the synthetic set. Real capability orders are selected
from convergence/interaction/performance fixtures, never from user list
order; this lexical pick is flagged for review when real adapters land.
"""

# Synthetic tool ID to capability to supported semantic file classes.
# Mirrors the `//quality:fixture_policy` shape: `lint-a` is selected by two
# families (proves cross-family union into one stage), `lint-b` and `fmt-a`
# are rust-only (prove exact subsets and python exclusion from format).
SYNTHETIC_ADAPTERS = {
    "fmt-a": {"format": ["rust"]},
    "lint-a": {"lint": ["python", "rust"]},
    "lint-b": {"lint": ["rust"]},
}

# WP2 fixture class-to-family assignment. The full registry assignment stays
# pending the O15 review; this minimal map only authorizes the fixture
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

# Real initial-adapter capability manifests (M04 WP2-WP3, O20; M12 WP3 adds
# the rustc typecheck stage).
#
# Tool IDs are the stable built-in identifiers users select in policy
# families. Every class has exactly one tool per capability except
# Markdown lint, where the repo-owned `markdown_check` (link/structure)
# and Vale (prose style) run as two ordered stages over the same files;
# no virtual convergence across tools runs yet. The pipeline formula
# orders stages by sorted tool ID (the provisional O19 rule) when several
# apply to one target. Rust typechecking is the toolchain `rustc` itself
# (`rust_toolchain_rustc`), invoked as a lib-root metadata check with no
# config discovery; it is check-only and never applies suggestions.
REAL_ADAPTERS = {
    "buildifier": {"format": ["starlark"], "lint": ["starlark"]},
    "clippy": {"lint": ["rust"]},
    "markdown_check": {"lint": ["markdown"]},
    "rustc": {"typecheck": ["rust"]},
    "rustfmt": {"format": ["rust"]},
    "taplo": {"format": ["toml"], "lint": ["toml"]},
    "vale": {"lint": ["markdown"]},
}

# WP2 fixture class-to-family assignment for the M04 source classes. Like
# the synthetic map, this is a fixture, not the frozen taxonomy: the full
# registry assignment and curated defaults stay pending the O15/O17
# reviews.
REAL_CLASS_TO_FAMILY = {
    "markdown": "markdown",
    "rust": "rust",
    "starlark": "starlark",
    "toml": "toml",
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
