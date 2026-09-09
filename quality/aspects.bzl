"""Target-scoped capability aspects over synthetic adapters (M03 WP2c).

Each aspect visits targets carrying `QualitySourcesInfo` and registers one
exact-input pipeline action for its capability when the workspace policy
selects a nonempty synthetic stage with effective sources. The action runs
`//rust/quality_runner:quality_runner` over the exact direct-source subset
and emits one versioned `QualityResult` protobuf in the stable `dx_results`
output group. No invocation-wide aggregation exists.

Contract: `docs/quality/action-model.md`, `docs/quality/tool-integrations.md`,
`docs/quality/quality-sources.md#adapter-applicability`,
`docs/quality/quality-result-protocol.md#transport`.

Explicit policy attribute: aspects take `//quality:fixture_policy` by
default. Workspace-flag (`@rules_dx//config:workspace`) resolution to a
typed aggregate stays pending (the `//dx:config` placeholder must fail
fast); M04+ binds the canonical policy without changing this exact-subset
formula.

No generic fallback: only `QualitySourcesInfo.direct_sources` supplies
subjects. `srcs`, `deps`, `DefaultInfo` outputs, rule names, extensions,
and tags other than `no-<capability>` never affect applicability. Empty
stages create no action.
"""

load("//quality:sources.bzl", "QualitySourcesInfo")
load("//quality:policy.bzl", "QualityPolicyInfo")
load(
    "//quality:adapters.bzl",
    "SYNTHETIC_ADAPTERS",
    "SYNTHETIC_CLASS_TO_FAMILY",
)
load("//quality:pipeline.bzl", "resolve_pipeline")

def _capability_tags(rule_attr, capability):
    tags = getattr(rule_attr, "tags", [])
    return ("no-" + capability) in tags

def _family_selections(policy, capability):
    selections = {}
    for family_id in policy.families.keys():
        selections[family_id] = getattr(policy.families[family_id], capability)
    return selections

def _quality_pipeline_action(target, ctx, capability):
    if QualitySourcesInfo not in target:
        return []
    if _capability_tags(ctx.rule.attr, capability):
        return []
    info = target[QualitySourcesInfo]
    policy = ctx.attr._policy[QualityPolicyInfo]

    direct_files = {}
    direct_paths = {}
    path_to_file = {}
    for class_id in info.direct_sources.keys():
        files = info.direct_sources[class_id].to_list()
        direct_files[class_id] = files
        paths = sorted([f.short_path for f in files])
        direct_paths[class_id] = paths
        for f in files:
            if f.short_path not in path_to_file:
                path_to_file[f.short_path] = f

    target_classes = sorted(direct_files.keys())
    selections = _family_selections(policy, capability)
    resolved = resolve_pipeline(
        target_classes,
        direct_paths,
        capability,
        selections,
        SYNTHETIC_CLASS_TO_FAMILY,
        SYNTHETIC_ADAPTERS,
    )
    if len(resolved) == 0:
        return []

    out = ctx.actions.declare_file(target.label.name + "-" + capability + ".pb")

    union = {}
    for stage in resolved:
        for path in stage["sources"]:
            union[path] = True
    ordered_paths = sorted(union.keys())
    inputs = [path_to_file[path] for path in ordered_paths if path in path_to_file]

    args = ctx.actions.args()
    args.add("--producer", str(target.label))
    args.add("--capability", capability)
    args.add("--output", out.path)
    for stage in resolved:
        args.add(
            "--stage",
            stage["tool"] + ";" + ",".join(stage["classes"]) + ";" + ",".join(stage["sources"]),
        )
    for ws_path in ordered_paths:
        f = path_to_file.get(ws_path)
        if f != None:
            args.add("--source", ws_path + "=" + f.path)

    ctx.actions.run(
        executable = ctx.executable._runner,
        inputs = depset(inputs),
        outputs = [out],
        arguments = [args],
        mnemonic = "DxQuality" + capability.capitalize(),
        progress_message = "Dx quality " + capability + " %{label}",
    )
    return [OutputGroupInfo(dx_results = depset([out]))]

def _lint_impl(target, ctx):
    return _quality_pipeline_action(target, ctx, "lint")

def _format_impl(target, ctx):
    return _quality_pipeline_action(target, ctx, "format")

def _typecheck_impl(target, ctx):
    return _quality_pipeline_action(target, ctx, "typecheck")

def _audit_impl(target, ctx):
    return _quality_pipeline_action(target, ctx, "audit")

_COMMON_ATTRS = {
    "_policy": attr.label(
        default = "//quality:fixture_policy",
        providers = [QualityPolicyInfo],
        doc = "Aggregate workspace policy expanding tool IDs to classes.",
    ),
    "_runner": attr.label(
        default = "//rust/quality_runner:quality_runner",
        executable = True,
        cfg = "exec",
        allow_files = True,
        doc = "Deterministic pipeline runner producing QualityResult protobufs.",
    ),
}

lint_aspect = aspect(
    implementation = _lint_impl,
    attr_aspects = [],
    attrs = _COMMON_ATTRS,
    doc = "Registers the exact-input lint pipeline action in dx_results.",
)

format_aspect = aspect(
    implementation = _format_impl,
    attr_aspects = [],
    attrs = _COMMON_ATTRS,
    doc = "Registers the exact-input format pipeline action in dx_results.",
)

typecheck_aspect = aspect(
    implementation = _typecheck_impl,
    attr_aspects = [],
    attrs = _COMMON_ATTRS,
    doc = "Registers the exact-input typecheck pipeline action in dx_results.",
)

audit_aspect = aspect(
    implementation = _audit_impl,
    attr_aspects = [],
    attrs = _COMMON_ATTRS,
    doc = "Registers independent audit actions in dx_results.",
)
