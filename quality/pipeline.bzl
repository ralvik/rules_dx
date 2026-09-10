"""Pure pipeline-construction helpers (M03 WP2).

Implements the `docs/quality/quality-sources.md#adapter-applicability`
formula in plain string shapes so `starlark_test` unit mode can pin it:

```text
effective classes =
    target provider classes
    intersect adapter-supported classes for the capability
    intersect classes allowed by workspace policy
```

Consuming aspects (WP2c) flatten `QualitySourcesInfo` and
`QualityPolicyInfo` into these shapes, then union the target's direct
sources over the effective classes themselves. Unknown selected tool IDs
fail during analysis, which the tool-integrations contract accepts as
"during configuration or analysis".

Stage order is the sorted tool-ID order (see `adapters.bzl` O19 note).
All outputs are JSON-encodable (dicts, lists, strings) for `expect_equal`.
"""

load(":applicability.bzl", "effective_classes")

def authorize_classes(family_selections, class_to_family):
    """Expands per-family tool selections into tool to authorizing classes.

    `family_selections` maps family ID to tool-ID list for one capability.
    `class_to_family` maps semantic class to owning family ID (WP2 fixture
    map). Returns a dict of tool ID to sorted authorizing class list: the
    union of classes assigned to every family selecting that tool. Several
    families selecting one adapter yield one stage over the union.

    Args:
      family_selections: maps family ID to tool-ID list for one capability.
      class_to_family: maps semantic class to owning family ID.

    Returns:
      Dict of tool ID to sorted authorizing class list.
    """
    family_to_classes = {}
    for class_id in class_to_family.keys():
        family_id = class_to_family[class_id]
        if family_id not in family_to_classes:
            family_to_classes[family_id] = {}
        family_to_classes[family_id][class_id] = True
    authorized = {}
    for family_id in family_selections.keys():
        tools = family_selections[family_id]
        classes = sorted(family_to_classes.get(family_id, {}).keys())
        for tool in tools:
            if tool not in authorized:
                authorized[tool] = {}
            for c in classes:
                authorized[tool][c] = True
    result = {}
    for tool in authorized.keys():
        result[tool] = sorted(authorized[tool].keys())
    return result

def stage_sources(direct_sources, effective_classes):
    """Unions direct-source paths over the effective classes, sorted.

    `direct_sources` maps class ID to path list. Missing classes contribute
    nothing. Output is sorted and deduplicated: declaration and depset order
    have no semantics.

    Args:
      direct_sources: maps class ID to source path list.
      effective_classes: class IDs whose sources participate.

    Returns:
      Sorted deduplicated source paths.
    """
    seen = {}
    for class_id in effective_classes:
        for path in direct_sources.get(class_id, []):
            seen[path] = True
    return sorted(seen.keys())

def pipeline_stages(target_classes, capability, family_selections, class_to_family, adapters):
    """Returns the nonempty stages for one target/capability, in tool order.

    Each stage is a dict `{"tool": ..., "classes": [...]}` with sorted
    effective classes. Adapters with no effective classes create no stage.
    Fails on a selected tool ID absent from `adapters`.

    Args:
      target_classes: provider classes of the target under audit.
      capability: capability name selecting the adapter support records.
      family_selections: maps family ID to tool-ID list for the capability.
      class_to_family: maps semantic class to owning family ID.
      adapters: maps known tool ID to its support record.

    Returns:
      Stage dicts with sorted effective classes, in tool order.
    """
    authorized = authorize_classes(family_selections, class_to_family)
    stages = []
    for tool in sorted(authorized.keys()):
        if tool not in adapters:
            fail("pipeline: unknown tool '" + tool +
                 "': not in the ruleset adapter registry")
        adapter_classes = adapters[tool].get(capability, [])
        effective = effective_classes(target_classes, adapter_classes, authorized[tool])
        if len(effective) > 0:
            stages.append({"classes": effective, "tool": tool})
    return stages

def resolve_pipeline(target_classes, direct_sources, capability, family_selections, class_to_family, adapters):
    """Returns stages with exact source subsets for one target/capability.

    Each entry is `{"tool": ..., "classes": [...], "sources": [...]}` with
    sorted classes and sorted deduplicated workspace-relative paths. Stages
    with no effective sources are omitted, so no empty action is registered.

    Args:
      target_classes: provider classes of the target under audit.
      direct_sources: maps class ID to source path list.
      capability: capability name selecting the adapter support records.
      family_selections: maps family ID to tool-ID list for the capability.
      class_to_family: maps semantic class to owning family ID.
      adapters: maps known tool ID to its support record.

    Returns:
      Stage dicts with sorted classes and exact source subsets.
    """
    resolved = []
    for stage in pipeline_stages(target_classes, capability, family_selections, class_to_family, adapters):
        sources = stage_sources(direct_sources, stage["classes"])
        if len(sources) > 0:
            resolved.append({
                "classes": stage["classes"],
                "sources": sources,
                "tool": stage["tool"],
            })
    return resolved
