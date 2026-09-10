"""Quality applicability helpers (M03 freeze).

Pure functions implementing the effective-class formula from
`docs/quality/quality-sources.md#adapter-applicability`:

```text
effective classes =
    target provider classes
    intersect adapter-supported classes for the capability
    intersect classes allowed by workspace policy
```

All inputs and outputs are plain string values so `starlark_test` unit
mode can pin them. Consuming aspects (WP2) flatten providers into these
shapes, then union the target's direct sources over the effective classes
themselves. Unknown selected tool IDs fail here during analysis, which the
tool-integrations contract accepts as "during configuration or analysis".
The class-to-policy-family assignment table is not frozen here; WP2
fixtures declare their own.
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
      selection: tool-ID list in workspace policy order.
      adapters: maps known tool ID to its support record.

    Returns:
      The selection in policy order, deduplicated.
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
      target_classes: provider classes of the target under audit.
      adapter_classes: classes the adapter supports for the capability.
      policy_classes: classes allowed by workspace policy.

    Returns:
      Sorted intersection of the three class lists.
    """
    adapter_set = {c: True for c in adapter_classes}
    policy_set = {c: True for c in policy_classes}
    effective = {}
    for c in target_classes:
        if c in adapter_set and c in policy_set:
            effective[c] = True
    return sorted(effective.keys())
