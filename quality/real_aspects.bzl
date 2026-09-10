"""Target-scoped real capability aspects over initial adapters (M04 WP2).

Each aspect visits targets carrying `QualitySourcesInfo` and registers one
exact-input pipeline action for its capability when the real fixture policy
selects a nonempty real stage with effective sources. The action runs
`//quality/runner:quality_runner` with `--real` over the exact
direct-source subset plus the resolved native configs and emits one
versioned `QualityResult` protobuf in the stable `dx_results` output group.
No invocation-wide aggregation exists.

Native configuration: source targets opt in through `aspect_hints`
carrying `DxNativeConfigInfo`. Without a hint the adapter runs pinned
upstream defaults, except Vale, which has no usable default and fails
analysis with guidance to supply and bind native policy.

Hermeticity: actions declare exactly the direct sources, the hinted config
closures, and the stage tool binaries as inputs. The runner materializes
exact bytes into a fresh scratch tree, never observes VCS state, and
launches tools with an empty `PATH` (see `quality_adapter::exec`).

Contract: `docs/quality/tool-integrations.md`,
`docs/quality/native-configuration.md`,
`docs/quality/quality-sources.md#adapter-applicability`,
`docs/quality/quality-result-protocol.md#transport`.
"""

load(
    "//quality:adapters.bzl",
    "REAL_ADAPTERS",
    "REAL_CLASS_TO_FAMILY",
)
load("//quality:native_config.bzl", "DxNativeConfigInfo", "collect_native_configs")
load("//quality:pipeline.bzl", "resolve_pipeline")
load("//quality:policy.bzl", "QualityPolicyInfo")
load("//quality:sources.bzl", "QualitySourcesInfo")
load("//rust/toolchains:bindings.bzl", "rust_toolchain_toolchains", "rust_toolchain_tools")

def _capability_tags(rule_attr, capability):
    tags = getattr(rule_attr, "tags", [])
    return ("no-" + capability) in tags

def _family_selections(policy, capability):
    selections = {}
    for family_id in policy.families.keys():
        selections[family_id] = getattr(policy.families[family_id], capability)
    return selections

def _real_pipeline_action(target, ctx, capability):
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
        REAL_CLASS_TO_FAMILY,
        REAL_ADAPTERS,
    )
    if len(resolved) == 0:
        return []

    hints = []
    if hasattr(ctx.rule.attr, "aspect_hints"):
        for hint_target in ctx.rule.attr.aspect_hints:
            if DxNativeConfigInfo in hint_target:
                hints.append(hint_target[DxNativeConfigInfo])
            else:
                fail("real_aspect (" + str(target.label) + "): aspect_hints must provide DxNativeConfigInfo")
    stage_tools = [stage["tool"] for stage in resolved]
    configs_by_tool = collect_native_configs(hints, stage_tools, str(target.label))

    for stage in resolved:
        if stage["tool"] == "vale" and stage["tool"] not in configs_by_tool:
            fail("real_aspect (" + str(target.label) + "): applicable Vale requires declared config; supply and bind native policy via aspect_hints (no usable upstream default)")

    clippy_driver, rustfmt = rust_toolchain_tools(ctx)
    tool_binaries = {
        "buildifier": ctx.file._buildifier,
        "clippy": clippy_driver,
        "markdown_check": ctx.file._markdown_check,
        "rustfmt": rustfmt,
        "taplo": ctx.file._taplo,
        "vale": ctx.file._vale,
    }

    out = ctx.actions.declare_file(target.label.name + "-real-" + capability + ".pb")

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
    args.add("--real")
    for tool in stage_tools:
        binary = tool_binaries[tool]
        args.add("--tool-binary", tool + "=" + binary.path)
        inputs.append(binary)
        if tool in configs_by_tool:
            hint = configs_by_tool[tool]
            config_rel = hint.config.short_path
            args.add("--tool-config", tool + "=" + config_rel)
            for f in hint.closure.to_list():
                args.add("--tool-file", tool + "=" + f.short_path + "=" + f.path)
                inputs.append(f)

    ctx.actions.run(
        executable = ctx.executable._runner,
        inputs = depset(inputs),
        outputs = [out],
        arguments = [args],
        mnemonic = "DxRealQuality" + capability.capitalize(),
        progress_message = "Dx real quality " + capability + " %{label}",
    )

    return [OutputGroupInfo(dx_results = depset([out]))]

def _real_lint_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "lint")

def _real_format_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "format")

_REAL_ATTRS = {
    "_policy": attr.label(
        default = "//quality:real_fixture_policy",
        providers = [QualityPolicyInfo],
        doc = "Aggregate workspace policy expanding tool IDs to classes.",
    ),
    "_runner": attr.label(
        default = "//quality/runner:quality_runner",
        executable = True,
        cfg = "exec",
        allow_files = True,
        doc = "Deterministic pipeline runner with --real backend.",
    ),
    "_buildifier": attr.label(
        default = "@dx_tools//:buildifier",
        allow_single_file = True,
        cfg = "exec",
        doc = "Pinned Buildifier artifact for Starlark pipelines.",
    ),
    "_markdown_check": attr.label(
        default = "//quality/markdown:quality_markdown",
        allow_single_file = True,
        cfg = "exec",
        doc = "Repo-owned Markdown link/structure checker for Markdown pipelines.",
    ),
    "_taplo": attr.label(
        default = "@dx_tools//:taplo",
        allow_single_file = True,
        cfg = "exec",
        doc = "Pinned Taplo artifact for TOML pipelines.",
    ),
    "_vale": attr.label(
        default = "@dx_tools//:vale",
        allow_single_file = True,
        cfg = "exec",
        doc = "Pinned Vale artifact for Markdown pipelines.",
    ),
}

real_lint_aspect = aspect(
    implementation = _real_lint_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_ATTRS,
    toolchains = rust_toolchain_toolchains(),
    doc = "Registers the exact-input real lint pipeline action in dx_results.",
)

real_format_aspect = aspect(
    implementation = _real_format_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_ATTRS,
    toolchains = rust_toolchain_toolchains(),
    doc = "Registers the exact-input real format pipeline action in dx_results.",
)
