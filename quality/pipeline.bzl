
load(":applicability.bzl", "effective_classes")

def authorize_classes(family_selections, class_to_family):
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
    seen = {}
    for class_id in effective_classes:
        for path in direct_sources.get(class_id, []):
            seen[path] = True
    return sorted(seen.keys())

def pipeline_stages(target_classes, capability, family_selections, class_to_family, adapters):
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
    seen = {}
    for class_id in target_classes:
        seen[class_id] = True
    return sorted(seen.keys())

def file_subject_paths(direct_sources):
    seen = {}
    for class_id in direct_sources.keys():
        for path in direct_sources[class_id]:
            seen[path] = True
    return sorted(seen.keys())

def depset_subject_paths(depset_lists):
    seen = {}
    for paths in depset_lists:
        for path in paths:
            seen[path] = True
    return sorted(seen.keys())

def runfiles_subject_paths(checked_paths, runfiles_paths):
    checked = {}
    for path in checked_paths:
        checked[path] = True
    seen = dict(checked)
    for path in runfiles_paths:
        if path not in checked:
            seen[path] = True
    return sorted(seen.keys())

def aspect_capability_blocked(rule_attr, capability):
    return ("no-" + capability) in getattr(rule_attr, "tags", [])

def aspect_family_selections(policy, capability):
    return {family_id: getattr(policy.families[family_id], capability) for family_id in policy.families.keys()}

def aspect_direct_maps(direct_sources, what):
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
    allow = {tool: True for tool in allowed_tools}
    return [stage for stage in resolved if stage["tool"] in allow]

def drop_pipeline_tool(resolved, tool):
    return [stage for stage in resolved if stage["tool"] != tool]

def ordered_pipeline_paths(resolved):
    union = {}
    for stage in resolved:
        for path in stage["sources"]:
            union[path] = True
    return sorted(union.keys())

def pipeline_inputs_for_paths(ordered_paths, path_to_file):
    return [path_to_file[path] for path in ordered_paths if path in path_to_file]

def stage_flag(stage):
    return stage["tool"] + ";" + ",".join(stage["classes"]) + ";" + ",".join(stage["sources"])

def prune_tool_generated_sources(resolved, generated_paths, tool):
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
    generated = {}
    for class_id in direct_files:
        for f in direct_files[class_id]:
            if not f.is_source:
                generated[f.short_path] = True
    return generated
