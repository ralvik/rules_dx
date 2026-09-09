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
