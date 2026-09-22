"""Pure pipeline-construction helpers (WP2).

Contract: `docs/quality/quality-sources.md#adapter-applicability`.
"""

load(":applicability.bzl", "effective_classes")

def authorize_classes(family_selections, class_to_family):
    """Expands per-family tool selections into tool to authorizing classes.

    `family_selections` maps family ID to tool-ID list for one capability.
    `class_to_family` maps semantic class to owning family ID (WP2 fixture
    map). Returns a dict of tool ID to sorted authorizing class list: the
    union of classes assigned to every family selecting that tool. Several
    families selecting one adapter yield one stage over the union."""
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
    have no semantics."""
    seen = {}
    for class_id in effective_classes:
        for path in direct_sources.get(class_id, []):
            seen[path] = True
    return sorted(seen.keys())

def pipeline_stages(target_classes, capability, family_selections, class_to_family, adapters):
    """Returns the nonempty stages for one target/capability, in ruleset order.

    Order follows the adapter registry definition order (`adapters` dict
    insertion order: `quality/adapters.bzl` for real/synthetic), which is the
    curated stable pipeline order, independent of user list order and not a
    bare lexical sort. Curated defaults (`quality/curated_defaults.bzl`) are
    in this same stable order. See: `docs/quality/tool-integrations.md`,
    `docs/decisions/0003-action-granularity.md`.
    Each stage is a dict `{"tool": ..., "classes": [...]}` with sorted
    effective classes. Adapters with no effective classes create no stage.
    Fails on a selected tool ID absent from `adapters`."""
    authorized = authorize_classes(family_selections, class_to_family)
    for tool in authorized.keys():
        if tool not in adapters:
            fail("pipeline: unknown tool '" + tool +
                 "': not in the ruleset adapter registry")
    stages = []
    for tool in adapters.keys():
        if tool not in authorized:
            continue
        adapter_classes = adapters[tool].get(capability, [])
        effective = effective_classes(target_classes, adapter_classes, authorized[tool])
        if len(effective) > 0:
            stages.append({"classes": effective, "tool": tool})
    return stages

def resolve_pipeline(target_classes, direct_sources, capability, family_selections, class_to_family, adapters):
    """Returns stages with exact source subsets for one target/capability.

    Each entry is `{"tool": ..., "classes": [...], "sources": [...]}` with
    sorted classes and sorted deduplicated workspace-relative paths. Stages
    with no effective sources are omitted, so no empty action is registered."""
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

def aspect_capability_blocked(rule_attr, capability):
    """Reports whether `no-<capability>` blocks the aspect (See: quality-sources.md#tags)."""
    return ("no-" + capability) in getattr(rule_attr, "tags", [])

def aspect_family_selections(policy, capability):
    """Expands one capability across policy families (See: quality-sources.md#adapter-applicability)."""
    return {family_id: getattr(policy.families[family_id], capability) for family_id in policy.families.keys()}

def aspect_direct_maps(direct_sources, what):
    """Splits provider sources into class/file/path maps (See: quality-sources.md#adapter-applicability).

    Fail-closed multi-config boundary: duplicate workspace paths across
    classes and `..` escapes fail analysis instead of first-wins or late
    runner rejection. See: `docs/quality/native-configuration.md#closures-and-action-inputs`.
    """
    direct_files = {}
    direct_paths = {}
    path_to_file = {}
    for class_id in direct_sources.keys():
        files = direct_sources[class_id].to_list()
        direct_files[class_id] = files
        paths = sorted([f.short_path for f in files])
        direct_paths[class_id] = paths
        for f in files:
            if ".." in f.short_path.split("/"):
                fail(what + ": source path escapes workspace: '" + f.short_path + "'")
            if f.short_path in path_to_file:
                fail(what + ": duplicate source path '" + f.short_path + "' across classes; one target/capability pipeline needs one owner per path")
            path_to_file[f.short_path] = f
    return (sorted(direct_files.keys()), direct_files, direct_paths, path_to_file)
