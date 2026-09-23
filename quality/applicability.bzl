"""Quality applicability helpers (freeze).

Contract: `docs/quality/quality-sources.md#adapter-applicability`.
"""

load(":policy.bzl", "CAPABILITIES")

def capability_selection(tools_by_capability, capability):
    """Returns the policy-ordered tool-ID selection for one capability.

    `tools_by_capability` maps capability name to tool-ID list. Fails on an
    unknown capability; a missing key means that capability is unselected.
    """
    if capability not in CAPABILITIES:
        fail("applicability: unknown capability '" + capability +
             "': want one of " + ", ".join(CAPABILITIES))
    return tools_by_capability.get(capability, [])

def selected_adapters(selection, adapters):
    """Resolves a policy-ordered tool-ID selection against adapter metadata.

    `adapters` maps known tool ID to its support record. Returns the
    selection in policy order, deduplicated. Fails on an unknown tool ID.

    Args:
      selection: Policy-ordered tool IDs to resolve.
      adapters: Maps known tool ID to its support record.

    Returns:
      Deduplicated tool IDs in policy order.
    """
    ordered = []
    seen = {}
    for tool_id in selection:
        if tool_id not in adapters:
            fail("applicability: unknown tool '" + tool_id +
                 "': not in the ruleset adapter registry")
        if tool_id not in seen:
            seen[tool_id] = True
            ordered.append(tool_id)
    return ordered

def effective_classes(target_classes, adapter_classes, policy_classes):
    """Returns the sorted three-way class intersection for one tool stage.

    Args:
      target_classes: Semantic classes present on the target.
      adapter_classes: Classes the adapter supports for the capability.
      policy_classes: Classes authorized by the policy for the tool.

    Returns:
      Sorted class IDs in all three sets.
    """
    adapter_set = {c: True for c in adapter_classes}
    policy_set = {c: True for c in policy_classes}
    effective = {}
    for c in target_classes:
        if c in adapter_set and c in policy_set:
            effective[c] = True
    return sorted(effective.keys())
