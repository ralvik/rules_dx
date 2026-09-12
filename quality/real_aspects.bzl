"""Target-scoped real capability aspects over real adapters (M04 WP2, M15 Python).

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
closures, the Markdown link-resolution siblings, and the stage tool
binaries as inputs. The runner materializes
exact bytes into a fresh scratch tree, never observes VCS state, and
launches tools with an empty `PATH` (see `quality_adapter::exec`).
Python venv launchers (pydoclint, flake8, pylint) additionally resolve
their runtime
through the runner's runfiles forest via `RUNFILES_DIR`; the forest is
action-local and never enters findings. Siblings (`markdown_siblings`
on the visited rule) resolve Markdown link targets only: they are never
linted and never enter findings or snapshots.

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
load("//rust/toolchains:bindings.bzl", "rust_toolchain_rustc", "rust_toolchain_toolchains", "rust_toolchain_tools")

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
        "flake8": ctx.executable._flake8,
        "markdown_check": ctx.file._markdown_check,
        "pydoclint": ctx.executable._pydoclint,
        "pylint": ctx.executable._pylint,
        "ruff": ctx.file._ruff,
        "rustc": rust_toolchain_rustc(ctx),
        "rustfmt": rustfmt,
        "taplo": ctx.file._taplo,
        "ty": ctx.file._ty,
        "vale": ctx.file._vale,
    }

    out = ctx.actions.declare_file(target.label.name + "-real-" + capability + ".pb")

    union = {}
    for stage in resolved:
        for path in stage["sources"]:
            union[path] = True
    ordered_paths = sorted(union.keys())
    inputs = [path_to_file[path] for path in ordered_paths if path in path_to_file]

    # Markdown link-resolution siblings: unclassified files declared via
    # `markdown_siblings` on the visited rule. Only collected when a
    # markdown_check stage runs, so non-Markdown actions stay
    # byte-identical. A sibling shadowing a checked source is dropped
    # (the checked source wins); siblings shadowing each other keep the
    # first in attribute order.
    sibling_pairs = {}
    if "markdown_check" in stage_tools and hasattr(ctx.rule.attr, "markdown_siblings"):
        for sibling in ctx.rule.attr.markdown_siblings:
            # Label entries may be rule targets (with a files depset) or
            # source files directly.
            if hasattr(sibling, "files"):
                sibling_files = sibling.files.to_list()
            else:
                sibling_files = [sibling]
            for f in sibling_files:
                if f.short_path not in path_to_file and f.short_path not in sibling_pairs:
                    sibling_pairs[f.short_path] = f

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
    for ws_path in sorted(sibling_pairs.keys()):
        f = sibling_pairs[ws_path]
        args.add("--sibling", ws_path + "=" + f.path)
        inputs.append(f)
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

    # The Python venv launchers (pydoclint, flake8, pylint) are static
    # stubs that locate their interpreter and site-packages through the
    # runfiles forest (adjacent `<stub>.runfiles/`, else `RUNFILES_DIR`):
    # the loose closure files alone leave them unable to initialize.
    # Staging a launcher as a tool merges its runfiles into the runner's
    # forest, and `RUNFILES_DIR` points the stub at that forest. The
    # directory is action-local and transient; it never enters findings
    # or snapshots.
    run_tools = []
    if "pydoclint" in stage_tools:
        run_tools.append(ctx.attr._pydoclint[DefaultInfo].files_to_run)
        args.add(
            "--tool-env",
            "pydoclint=RUNFILES_DIR=" + ctx.executable._runner.path + ".runfiles",
        )
    if "flake8" in stage_tools:
        run_tools.append(ctx.attr._flake8[DefaultInfo].files_to_run)
        args.add(
            "--tool-env",
            "flake8=RUNFILES_DIR=" + ctx.executable._runner.path + ".runfiles",
        )
    if "pylint" in stage_tools:
        run_tools.append(ctx.attr._pylint[DefaultInfo].files_to_run)
        args.add(
            "--tool-env",
            "pylint=RUNFILES_DIR=" + ctx.executable._runner.path + ".runfiles",
        )

    ctx.actions.run(
        executable = ctx.executable._runner,
        inputs = depset(inputs),
        tools = run_tools,
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

def _real_typecheck_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "typecheck")

_REAL_ATTRS = {
    "_buildifier": attr.label(
        default = "@dx_tools//:buildifier",
        allow_single_file = True,
        cfg = "exec",
        doc = "Pinned Buildifier artifact for Starlark pipelines.",
    ),
    "_flake8": attr.label(
        default = "//quality/tools/python:flake8",
        cfg = "exec",
        executable = True,
        doc = "Pinned flake8 launcher (stub plus runfiles closure) for Python lint opt-ins.",
    ),
    "_markdown_check": attr.label(
        default = "//quality/markdown:quality_markdown",
        allow_single_file = True,
        cfg = "exec",
        doc = "Repo-owned Markdown link/structure checker for Markdown pipelines.",
    ),
    "_policy": attr.label(
        default = "//quality:real_fixture_policy",
        providers = [QualityPolicyInfo],
        doc = "Aggregate workspace policy expanding tool IDs to classes.",
    ),
    "_pydoclint": attr.label(
        default = "//quality/tools/python:pydoclint",
        cfg = "exec",
        executable = True,
        doc = "Pinned pydoclint launcher (stub plus runfiles closure) for Python pipelines.",
    ),
    "_pylint": attr.label(
        default = "//quality/tools/python:pylint",
        cfg = "exec",
        executable = True,
        doc = "Pinned pylint launcher (stub plus runfiles closure) for Python lint opt-ins.",
    ),
    "_ruff": attr.label(
        default = "@dx_tools//:ruff",
        allow_single_file = True,
        cfg = "exec",
        doc = "Pinned Ruff artifact for Python pipelines.",
    ),
    "_runner": attr.label(
        default = "//quality/runner:quality_runner",
        executable = True,
        cfg = "exec",
        allow_files = True,
        doc = "Deterministic pipeline runner with --real backend.",
    ),
    "_taplo": attr.label(
        default = "@dx_tools//:taplo",
        allow_single_file = True,
        cfg = "exec",
        doc = "Pinned Taplo artifact for TOML pipelines.",
    ),
    "_ty": attr.label(
        default = "@dx_tools//:ty",
        allow_single_file = True,
        cfg = "exec",
        doc = "Pinned Ty artifact for Python pipelines.",
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

real_typecheck_aspect = aspect(
    implementation = _real_typecheck_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_ATTRS,
    toolchains = rust_toolchain_toolchains(),
    doc = "Registers the exact-input real typecheck pipeline action in dx_results.",
)
