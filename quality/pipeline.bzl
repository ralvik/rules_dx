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
    families selecting one adapter yield one stage over the union.

    Args:
      family_selections: Maps family ID to selected tool-ID list for one capability.
      class_to_family: Maps semantic class to owning family ID.

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
      direct_sources: Maps class ID to path list.
      effective_classes: Class IDs whose paths are included.

    Returns:
      Sorted deduplicated workspace-relative paths.
    """
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
    Fails on a selected tool ID absent from `adapters`.

    Args:
      target_classes: Semantic classes present on the target.
      capability: Capability name (e.g. `lint`, `format`).
      family_selections: Maps family ID to selected tool-ID list.
      class_to_family: Maps semantic class to owning family ID.
      adapters: Adapter registry mapping tool ID to capability metadata.

    Returns:
      Nonempty stage dicts in adapter-registry order.
    """
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
    with no effective sources are omitted, so no empty action is registered.

    Args:
      target_classes: Semantic classes present on the target.
      direct_sources: Maps class ID to path list (or File lists by short_path).
      capability: Capability name (e.g. `lint`, `format`).
      family_selections: Maps family ID to selected tool-ID list.
      class_to_family: Maps semantic class to owning family ID.
      adapters: Adapter registry mapping tool ID to capability metadata.

    Returns:
      Stage dicts with tool, sorted classes, and sorted source paths.
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

def target_subject_classes(target_classes):
    """Canonicalizes target classes for custom-rule subject assertions.

    Sorts and dedupes the class list so custom rules testing
    QualitySourcesInfo construction (ADR 0002/0011) can assert target
    subjects deterministically regardless of declaration order.

    Args:
      target_classes: Class IDs to canonicalize.

    Returns:
      Sorted deduplicated class list.
    """
    seen = {}
    for class_id in target_classes:
        seen[class_id] = True
    return sorted(seen.keys())

def file_subject_paths(direct_sources):
    """Unions direct-source paths for custom-rule file-subject assertions.

    `direct_sources` maps class ID to path list (string test double for
    the provider depsets). Output is sorted and deduplicated so custom
    rules can assert file subjects without Bazel File objects.

    Args:
      direct_sources: Maps class ID to path list.

    Returns:
      Sorted deduplicated workspace-relative paths.
    """
    seen = {}
    for class_id in direct_sources.keys():
        for path in direct_sources[class_id]:
            seen[path] = True
    return sorted(seen.keys())

def depset_subject_paths(depset_lists):
    """Unions depset test doubles for custom-rule depset assertions.

    `depset_lists` is a list of path lists, each standing in for one
    depset's `to_list()`. Output is sorted and deduplicated so custom
    rules can assert depset unions without Bazel depset objects.

    Args:
      depset_lists: List of path lists, each standing in for one depset.

    Returns:
      Sorted deduplicated union of all path lists.
    """
    seen = {}
    for paths in depset_lists:
        for path in paths:
            seen[path] = True
    return sorted(seen.keys())

def runfiles_subject_paths(checked_paths, runfiles_paths):
    """Unions runfiles for custom-rule runfiles-subject assertions.

    Checked sources win on shadow (mirroring the Markdown sibling rule),
    so a runfiles path colliding with a checked path is dropped. Output
    is sorted so custom rules can assert runfiles subjects
    deterministically.

    Args:
      checked_paths: Checked source paths that win on shadow.
      runfiles_paths: Runfiles paths unioned when not shadowed.

    Returns:
      Sorted deduplicated runfiles subject paths.
    """
    checked = {}
    for path in checked_paths:
        checked[path] = True
    seen = dict(checked)
    for path in runfiles_paths:
        if path not in checked:
            seen[path] = True
    return sorted(seen.keys())

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

    Args:
      direct_sources: Maps class ID to provider file depsets.
      what: Diagnostic prefix naming the failing target or context.

    Returns:
      Tuple of (sorted class IDs, class-to-files, class-to-paths, path-to-file).
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

def filter_pipeline_by_tools(resolved, allowed_tools):
    """Keeps only stages whose tool is in `allowed_tools` (See: quality-sources.md#adapter-applicability)."""
    allow = {tool: True for tool in allowed_tools}
    return [stage for stage in resolved if stage["tool"] in allow]

def drop_pipeline_tool(resolved, tool):
    """Drops one target-coupled tool from resolved stages (See: tool-integrations.md)."""
    return [stage for stage in resolved if stage["tool"] != tool]

def ordered_pipeline_paths(resolved):
    """Unions resolved stage sources into sorted workspace paths (See: action-model.md#outputs-remote-cache-and-execution).

    Args:
      resolved: Resolved stage dicts with `sources` path lists.

    Returns:
      Sorted deduplicated workspace paths across all stages.
    """
    union = {}
    for stage in resolved:
        for path in stage["sources"]:
            union[path] = True
    return sorted(union.keys())

def pipeline_inputs_for_paths(ordered_paths, path_to_file):
    """Maps ordered workspace paths to action input files (See: action-model.md#outputs-remote-cache-and-execution)."""
    return [path_to_file[path] for path in ordered_paths if path in path_to_file]

def stage_flag(stage):
    """Renders one resolved stage as a `--stage` flag value (See: action-model.md#deterministic-arguments)."""
    return stage["tool"] + ";" + ",".join(stage["classes"]) + ";" + ",".join(stage["sources"])

def prune_tool_generated_sources(resolved, generated_paths, tool):
    """Drops generated paths from one tool's stages, omitting emptied stages (See: tool-integrations.md).

    Args:
      resolved: Resolved stage dicts to filter.
      generated_paths: Workspace paths of generated (non-source) files.
      tool: Tool ID whose stages lose generated paths.

    Returns:
      Filtered stage list with emptied stages omitted.
    """
    kept = []
    for stage in resolved:
        if stage["tool"] != tool:
            kept.append(stage)
            continue
        sources = [p for p in stage["sources"] if p not in generated_paths]
        if len(sources) > 0:
            pruned = dict(stage)
            pruned["sources"] = sources
            kept.append(pruned)
    return kept

def generated_source_paths(direct_files):
    """Collects non-source (`is_source == False`) workspace paths (See: tool-integrations.md).

    Args:
      direct_files: Maps class ID to File lists.

    Returns:
      Dict set of generated workspace paths.
    """
    generated = {}
    for class_id in direct_files:
        for f in direct_files[class_id]:
            if not f.is_source:
                generated[f.short_path] = True
    return generated
