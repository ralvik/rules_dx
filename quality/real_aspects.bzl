"""Target-scoped real capability aspects over real adapters.

Contract: `docs/quality/tool-integrations.md`, `docs/quality/native-configuration.md`, `docs/quality/quality-sources.md#adapter-applicability`, `docs/quality/quality-result-protocol.md#transport`.
"""

load("@aspect_rules_py//py:defs.bzl", _PyInfo = "PyInfo")
load("@rules_java//java/common:java_info.bzl", "JavaInfo")
load("@rules_rust//rust:defs.bzl", "rust_clippy_aspect", _rust_common = "rust_common")
load(
    "//quality:adapters.bzl",
    "REAL_ADAPTERS",
    "REAL_CLASS_TO_FAMILY",
)
load("//quality:native_config.bzl", "DxNativeConfigInfo", "collect_native_configs")
load("//quality:pipeline.bzl", "resolve_pipeline")
load("//quality:policy.bzl", "QualityPolicyInfo")
load("//quality:sources.bzl", "QualitySourcesInfo")
load("//rust/rules:edition.bzl", "RUST_EDITION")
load("//rust/toolchains:bindings.bzl", "rust_toolchain_rustc", "rust_toolchain_toolchains", "rust_toolchain_tools")

def _capability_tags(rule_attr, capability):
    tags = getattr(rule_attr, "tags", [])
    return ("no-" + capability) in tags

def _family_selections(policy, capability):
    selections = {}
    for family_id in policy.families.keys():
        selections[family_id] = getattr(policy.families[family_id], capability)
    return selections

# Cold-server laziness: the shared aspect no longer resolves every
# ecosystem on every visit. Each aspect declares only its own tool labels
# and filters the resolved pipeline to `allowed_tools`; `select()` and
# toolchain indirection cannot do this (all branches resolve), only separate
# aspects avoid loading. Base handles single-file `dx_tools` artifacts plus
# repo-owned markdown; JS/Python/Rust/JVM families are additive opt-ins.
# JVM tools run as `java_binary` wrappers over the complete upstream
# artifacts plus the shared managed JDK (remotejdk_21 via
# `--java_runtime_version`); SpotBugs is target-coupled (needs the
# authoritative `JavaInfo` classes, dropped for provider-less targets
# like tsc without `TsConfigInfo`).
_CORE_LINT_TOOLS = ["biome", "buildifier", "markdown_check", "ruff", "taplo", "vale"]
_CORE_FORMAT_TOOLS = ["biome", "buildifier", "ruff", "taplo"]
_CORE_TYPECHECK_TOOLS = ["ty"]
_JS_LINT_TOOLS = ["eslint"]
_JS_FORMAT_TOOLS = ["prettier"]
_PY_LINT_TOOLS = ["flake8", "pydoclint", "pylint"]
_RUST_LINT_TOOLS = ["clippy"]
_RUST_FORMAT_TOOLS = ["rustfmt"]
_RUST_TYPECHECK_TOOLS = ["rustc"]
_JVM_LINT_TOOLS = ["checkstyle", "ktlint", "pmd", "spotbugs"]
_JVM_FORMAT_TOOLS = ["google_java_format", "ktfmt"]

def _real_pipeline_action(target, ctx, capability, allowed_tools, output_suffix, has_rust_toolchain):
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
    resolved = [stage for stage in resolved if stage["tool"] in allowed_tools]
    if len(resolved) == 0:
        return []

    # Target-coupled tsc: tsc never applies from the class alone;
    # it requires the authoritative typescript_project context
    # (TsConfigInfo). Fixture QualitySourcesInfo-only targets carry no
    # TsConfigInfo, so drop tsc stages there (unfetched, keys unchanged
    # per the target-coupled laziness row). Authoritative targets delegate
    # tsc to the upstream build and test: `transpiler = "tsc"` fails the
    # `bazel build //...` (and `dx build //...`) compilation on type errors,
    # and `<name>_upstream_typecheck_test` runs under `bazel test //...`
    # (and `dx test //...`); see quality/tools/typescript/BUILD.bazel (never
    # a bare tsc file invocation, which would lose tsconfig/declaration
    # context). Drop tsc here so `dx typecheck --check //...` stays green
    # while TS type safety is proven by the build plus test checks.
    if "tsc" in [stage["tool"] for stage in resolved]:
        resolved = [stage for stage in resolved if stage["tool"] != "tsc"]
        if len(resolved) == 0:
            return []

    # Target-coupled SpotBugs: SpotBugs analyzes compiled classes, never
    # bare sources, so it requires the authoritative `JavaInfo` (its
    # `transitive_runtime_jars` feed the `-textui` analysis). Fixture
    # QualitySourcesInfo-only targets carry no `JavaInfo`, so drop
    # SpotBugs stages there (unfetched, keys unchanged per the
    # target-coupled laziness row). Authoritative `java_library` targets
    # keep their SpotBugs stage with the compiled closure as inputs.
    if "spotbugs" in [stage["tool"] for stage in resolved]:
        if JavaInfo not in target:
            resolved = [stage for stage in resolved if stage["tool"] != "spotbugs"]
            if len(resolved) == 0:
                return []

    # rustfmt crate context: the edition comes from the
    # authoritative `CrateInfo` (or the test crate's inner `CrateInfo`,
    # exactly as upstream `_get_rustfmt_ready_crate_info`), and generated
    # files never reach the tool (upstream formats `is_source` files
    # only). Provider-less fixture targets carry no crate context, so
    # they fall back to `RUST_EDITION` (single source of truth).
    # `no-format` skips the stage via `_capability_tags` above.
    rustfmt_edition = None
    if "rustfmt" in [stage["tool"] for stage in resolved]:
        if not has_rust_toolchain:
            fail("real_aspect (" + str(target.label) + "): rustfmt needs the Rust family aspect")
        if _rust_common.crate_info in target:
            rustfmt_edition = target[_rust_common.crate_info].edition
        elif _rust_common.test_crate_info in target:
            rustfmt_edition = target[_rust_common.test_crate_info].crate.edition
        else:
            rustfmt_edition = RUST_EDITION
        generated = {}
        for class_id in direct_files:
            for f in direct_files[class_id]:
                if not f.is_source:
                    generated[f.short_path] = True
        if len(generated) > 0:
            kept = []
            for stage in resolved:
                if stage["tool"] == "rustfmt":
                    sources = [p for p in stage["sources"] if p not in generated]
                    if len(sources) > 0:
                        pruned = dict(stage)
                        pruned["sources"] = sources
                        kept.append(pruned)
                else:
                    kept.append(stage)
            resolved = kept

    # Delegated Clippy: this aspect requires the upstream
    # `rust_clippy_aspect`, which emits the authoritative
    # `.clippy.diagnostics` file into the `clippy_output` output group
    # when `--@rules_rust//rust/settings:clippy_output_diagnostics` is
    # set (`dx lint` sets it; see `cli/cli/src/plan.rs`). The runner
    # parses that file instead of spawning Clippy, so dependency
    # context, edition, and crate type always match the real build.
    # Without the group (non-Rust-rule targets) the clippy stage runs
    # with no upstream file and reports no findings; the legacy
    # self-run path is gone, so Clippy takes no dx-side config.
    clippy_diagnostics = []
    if "clippy" in [stage["tool"] for stage in resolved]:
        if not has_rust_toolchain:
            fail("real_aspect (" + str(target.label) + "): clippy needs the Rust family aspect")
        if OutputGroupInfo in target and "clippy_output" in target[OutputGroupInfo]:
            clippy_diagnostics = target[OutputGroupInfo]["clippy_output"].to_list()
    clippy_delegated = len(clippy_diagnostics) > 0

    # Delegated rustc: every Rust rule emits the authoritative
    # `.rustc-output` JSON file into the `rustc_output` output group
    # when `--@rules_rust//rust/settings:rustc_output_diagnostics` is
    # set (`dx typecheck` sets it; see `cli/cli/src/plan.rs`). The
    # runner parses that file instead of spawning rustc, so dependency
    # context, edition, and crate type always match the real build.
    # Without the group (non-Rust-rule targets) the rustc stage runs
    # with no upstream file and reports no findings; the legacy
    # self-run path is gone, so rustc takes no dx-side invocation.
    rustc_diagnostics = []
    if "rustc" in [stage["tool"] for stage in resolved]:
        if not has_rust_toolchain:
            fail("real_aspect (" + str(target.label) + "): rustc needs the Rust family aspect")
        if OutputGroupInfo in target and "rustc_output" in target[OutputGroupInfo]:
            rustc_diagnostics = target[OutputGroupInfo]["rustc_output"].to_list()
    rustc_delegated = len(rustc_diagnostics) > 0

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
        if stage["tool"] == "checkstyle" and stage["tool"] not in configs_by_tool:
            fail("real_aspect (" + str(target.label) + "): applicable Checkstyle requires declared config; supply and bind native policy via aspect_hints (no usable upstream default)")

    tool_binaries = {}
    if "biome" in stage_tools:
        tool_binaries["biome"] = ctx.file._biome
    if "buildifier" in stage_tools:
        tool_binaries["buildifier"] = ctx.file._buildifier
    if "eslint" in stage_tools:
        tool_binaries["eslint"] = ctx.executable._eslint
    if "flake8" in stage_tools:
        tool_binaries["flake8"] = ctx.executable._flake8
    if "markdown_check" in stage_tools:
        tool_binaries["markdown_check"] = ctx.file._markdown_check
    if "prettier" in stage_tools:
        tool_binaries["prettier"] = ctx.executable._prettier
    if "pydoclint" in stage_tools:
        tool_binaries["pydoclint"] = ctx.executable._pydoclint
    if "pylint" in stage_tools:
        tool_binaries["pylint"] = ctx.executable._pylint
    if "ruff" in stage_tools:
        tool_binaries["ruff"] = ctx.file._ruff
    if "taplo" in stage_tools:
        tool_binaries["taplo"] = ctx.file._taplo
    if "ty" in stage_tools:
        tool_binaries["ty"] = ctx.file._ty
    if "vale" in stage_tools:
        tool_binaries["vale"] = ctx.file._vale
    if "google_java_format" in stage_tools:
        tool_binaries["google_java_format"] = ctx.executable._google_java_format
    if "ktfmt" in stage_tools:
        tool_binaries["ktfmt"] = ctx.executable._ktfmt
    if "checkstyle" in stage_tools:
        tool_binaries["checkstyle"] = ctx.executable._checkstyle
    if "pmd" in stage_tools:
        tool_binaries["pmd"] = ctx.executable._pmd
    if "spotbugs" in stage_tools:
        tool_binaries["spotbugs"] = ctx.executable._spotbugs
    if "ktlint" in stage_tools:
        tool_binaries["ktlint"] = ctx.executable._ktlint
    if has_rust_toolchain:
        clippy_driver, rustfmt = rust_toolchain_tools(ctx)
        if "clippy" in stage_tools and not clippy_delegated:
            tool_binaries["clippy"] = clippy_driver
        if "rustfmt" in stage_tools:
            tool_binaries["rustfmt"] = rustfmt
        if "rustc" in stage_tools and not rustc_delegated:
            tool_binaries["rustc"] = rust_toolchain_rustc(ctx)

    out = ctx.actions.declare_file(target.label.name + "-real-" + capability + output_suffix + ".pb")

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

    # Ty import context: ty resolves same-package imports through the
    # filesystem, but actions stage only direct sources, so `import handlers`
    # in a direct source fails when `handlers.py` comes from `deps`. The
    # Python forwarders preserve upstream `PyInfo`, whose transitive sources
    # cover deps; stage transitive-minus-direct Python sources as
    # resolution-only inputs (never checked, never reported; their own
    # targets' actions own their findings). The runner derives ty search
    # dirs from staged files. Transitive (not just direct) deps are covered
    # via `PyInfo`; non-Python targets without `PyInfo` simply stage nothing.
    resolve_pairs = {}
    if "ty" in stage_tools and _PyInfo in target:
        transitive = getattr(target[_PyInfo], "transitive_sources", None)
        if transitive != None:
            for f in transitive.to_list():
                ws_path = f.short_path
                if ws_path in path_to_file or ws_path in sibling_pairs or ws_path in resolve_pairs:
                    continue
                if not ws_path.endswith(".py") and not ws_path.endswith(".pyi"):
                    continue
                if ws_path.startswith("../"):
                    continue
                resolve_pairs[ws_path] = f
    for ws_path in sorted(resolve_pairs.keys()):
        f = resolve_pairs[ws_path]
        args.add("--resolve", ws_path + "=" + f.path)
        inputs.append(f)
    args.add("--real")
    for tool in stage_tools:
        if tool == "clippy" and clippy_delegated:
            # Delegated Clippy needs no spawned binary and no dx-side
            # policy: the upstream aspect owns the invocation and its
            # own `clippy.toml` label flag. The diagnostics file below
            # creates the runner tool entry.
            continue
        if tool == "rustc" and rustc_delegated:
            # Delegated rustc needs no spawned binary: the upstream
            # rule owns the compilation. The diagnostics file below
            # creates the runner tool entry.
            continue
        binary = tool_binaries[tool]
        args.add("--tool-binary", tool + "=" + binary.path)
        inputs.append(binary)
        if tool == "rustfmt":
            # Always set when a rustfmt stage survives: provider-less
            # targets fall back to RUST_EDITION above, so the runner's
            # mandatory edition always resolves.
            args.add("--tool-edition", "rustfmt=" + rustfmt_edition)
        if tool in configs_by_tool:
            hint = configs_by_tool[tool]
            config_rel = hint.config.short_path
            args.add("--tool-config", tool + "=" + config_rel)
            for f in hint.closure.to_list():
                args.add("--tool-file", tool + "=" + f.short_path + "=" + f.path)
                inputs.append(f)
    if clippy_delegated:
        for diagnostics in clippy_diagnostics:
            args.add("--upstream-diagnostics", "clippy=" + diagnostics.path)
            inputs.append(diagnostics)
    if rustc_delegated:
        for diagnostics in rustc_diagnostics:
            args.add("--upstream-diagnostics", "rustc=" + diagnostics.path)
            inputs.append(diagnostics)

    # The Python venv launchers (pydoclint, flake8, pylint) are static
    # stubs that locate their interpreter and site-packages through the
    # runfiles forest (adjacent `<stub>.runfiles/`, else `RUNFILES_DIR`):
    # the loose closure files alone leave them unable to initialize.
    # Staging a launcher as a tool merges its runfiles into the runner's
    # forest, and `RUNFILES_DIR` points the stub at that forest. The
    # directory is action-local and transient; it never enters findings
    # or snapshots. The Node js_binary wrappers (eslint, prettier) likewise
    # resolve their runtime through `$0.runfiles`, so they are staged as
    # tools; the runner spawns them from scratch trees under TMPDIR, so
    # `JS_BINARY__NO_CD_BINDIR=1` keeps the wrapper from changing directory
    # into BAZEL_BINDIR (the devserver uses the same flag for custom cwd).
    run_tools = []
    if "eslint" in stage_tools:
        run_tools.append(ctx.attr._eslint[DefaultInfo].files_to_run)
        args.add(
            "--tool-env",
            "eslint=JS_BINARY__NO_CD_BINDIR=1",
        )
    if "prettier" in stage_tools:
        run_tools.append(ctx.attr._prettier[DefaultInfo].files_to_run)
        args.add(
            "--tool-env",
            "prettier=JS_BINARY__NO_CD_BINDIR=1",
        )
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
    # JVM `java_binary` wrappers (google-java-format, Checkstyle, PMD,
    # SpotBugs, ktfmt, ktlint) locate their managed JDK plus tool JARs
    # through their runfiles forest (adjacent `$0.runfiles`), so each
    # staged wrapper gets its runfiles merged into the runner's forest
    # like the Python/Node launchers above. No extra tool-env: the
    # wrappers respect the runner's scratch cwd.
    if "google_java_format" in stage_tools:
        run_tools.append(ctx.attr._google_java_format[DefaultInfo].files_to_run)
    if "ktfmt" in stage_tools:
        run_tools.append(ctx.attr._ktfmt[DefaultInfo].files_to_run)
    if "checkstyle" in stage_tools:
        run_tools.append(ctx.attr._checkstyle[DefaultInfo].files_to_run)
    if "pmd" in stage_tools:
        run_tools.append(ctx.attr._pmd[DefaultInfo].files_to_run)
    if "spotbugs" in stage_tools:
        run_tools.append(ctx.attr._spotbugs[DefaultInfo].files_to_run)
    if "ktlint" in stage_tools:
        run_tools.append(ctx.attr._ktlint[DefaultInfo].files_to_run)

    ctx.actions.run(
        executable = ctx.executable._runner,
        inputs = depset(inputs),
        tools = run_tools,
        outputs = [out],
        arguments = [args],
        # Local-only until remote qualified (See: docs/quality/action-model.md#outputs-remote-cache-and-execution).
        execution_requirements = {"no-remote-exec": "1"},
        mnemonic = "DxRealQuality" + capability.capitalize(),
        progress_message = "Dx real quality " + capability + " %{label}",
    )

    return [OutputGroupInfo(dx_results = depset([out]))]

def _real_lint_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "lint", _CORE_LINT_TOOLS, "", False)

def _real_format_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "format", _CORE_FORMAT_TOOLS, "", False)

def _real_typecheck_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "typecheck", _CORE_TYPECHECK_TOOLS, "", False)

def _real_js_lint_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "lint", _JS_LINT_TOOLS, "-js", False)

def _real_js_format_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "format", _JS_FORMAT_TOOLS, "-js", False)

def _real_python_lint_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "lint", _PY_LINT_TOOLS, "-py", False)

def _real_jvm_lint_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "lint", _JVM_LINT_TOOLS, "-jvm", False)

def _real_jvm_format_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "format", _JVM_FORMAT_TOOLS, "-jvm", False)

def _real_rust_lint_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "lint", _RUST_LINT_TOOLS, "-rust", True)

def _real_rust_format_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "format", _RUST_FORMAT_TOOLS, "-rust", True)

def _real_rust_typecheck_impl(target, ctx):
    return _real_pipeline_action(target, ctx, "typecheck", _RUST_TYPECHECK_TOOLS, "-rust", True)

_REAL_CORE_ATTRS = {
    "_biome": attr.label(
        default = "@dx_tools//:biome",
        allow_single_file = True,
        cfg = "exec",
        doc = "Pinned Biome standalone artifact for JavaScript/TypeScript/JSON pipelines.",
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
    "_policy": attr.label(
        default = "//quality:real_fixture_policy",
        providers = [QualityPolicyInfo],
        doc = "Aggregate workspace policy expanding tool IDs to classes.",
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

_REAL_JS_LINT_ATTRS = {
    "_eslint": attr.label(
        default = "//quality/tools/javascript/bin:eslint",
        cfg = "exec",
        executable = True,
        doc = "Private ESLint js_binary wrapper for JavaScript lint opt-ins.",
    ),
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
}

_REAL_JS_FORMAT_ATTRS = {
    "_policy": attr.label(
        default = "//quality:real_fixture_policy",
        providers = [QualityPolicyInfo],
        doc = "Aggregate workspace policy expanding tool IDs to classes.",
    ),
    "_prettier": attr.label(
        default = "//quality/tools/javascript/bin:prettier",
        cfg = "exec",
        executable = True,
        doc = "Private Prettier js_binary wrapper for JavaScript/JSON format.",
    ),
    "_runner": attr.label(
        default = "//quality/runner:quality_runner",
        executable = True,
        cfg = "exec",
        allow_files = True,
        doc = "Deterministic pipeline runner with --real backend.",
    ),
}

_REAL_JVM_LINT_ATTRS = {
    "_checkstyle": attr.label(
        default = "//quality/tools/jvm:checkstyle",
        cfg = "exec",
        executable = True,
        doc = "Pinned Checkstyle java_binary wrapper for Java lint.",
    ),
    "_ktlint": attr.label(
        default = "//quality/tools/jvm:ktlint",
        cfg = "exec",
        executable = True,
        doc = "Pinned ktlint java_binary wrapper for Kotlin lint (fixes via --format).",
    ),
    "_pmd": attr.label(
        default = "//quality/tools/jvm:pmd",
        cfg = "exec",
        executable = True,
        doc = "Pinned PMD java_binary wrapper for Java lint.",
    ),
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
    "_spotbugs": attr.label(
        default = "//quality/tools/jvm:spotbugs",
        cfg = "exec",
        executable = True,
        doc = "Pinned SpotBugs java_binary wrapper for Java lint (target-coupled via JavaInfo).",
    ),
}

_REAL_JVM_FORMAT_ATTRS = {
    "_google_java_format": attr.label(
        default = "//quality/tools/jvm:google_java_format",
        cfg = "exec",
        executable = True,
        doc = "Pinned google-java-format java_binary wrapper for Java format.",
    ),
    "_ktfmt": attr.label(
        default = "//quality/tools/jvm:ktfmt",
        cfg = "exec",
        executable = True,
        doc = "Pinned ktfmt java_binary wrapper for Kotlin format.",
    ),
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
}

_REAL_PY_LINT_ATTRS = {
    "_flake8": attr.label(
        default = "//quality/tools/python:flake8",
        cfg = "exec",
        executable = True,
        doc = "Pinned flake8 launcher (stub plus runfiles closure) for Python lint opt-ins.",
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
    "_runner": attr.label(
        default = "//quality/runner:quality_runner",
        executable = True,
        cfg = "exec",
        allow_files = True,
        doc = "Deterministic pipeline runner with --real backend.",
    ),
}

_REAL_RUST_ATTRS = {
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
}

real_lint_aspect = aspect(
    implementation = _real_lint_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_CORE_ATTRS,
    doc = "Registers the exact-input real lint pipeline action in dx_results.",
)

real_format_aspect = aspect(
    implementation = _real_format_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_CORE_ATTRS,
    doc = "Registers the exact-input real format pipeline action in dx_results.",
)

real_typecheck_aspect = aspect(
    implementation = _real_typecheck_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_CORE_ATTRS,
    doc = "Registers the exact-input real typecheck pipeline action in dx_results.",
)

real_js_lint_aspect = aspect(
    implementation = _real_js_lint_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_JS_LINT_ATTRS,
    doc = "Additive JavaScript lint family aspect (ESLint opt-in).",
)

real_js_format_aspect = aspect(
    implementation = _real_js_format_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_JS_FORMAT_ATTRS,
    doc = "Additive JavaScript/JSON format family aspect (Prettier).",
)

real_python_lint_aspect = aspect(
    implementation = _real_python_lint_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_PY_LINT_ATTRS,
    doc = "Additive Python lint family aspect (flake8/pylint/pydoclint).",
)

real_jvm_lint_aspect = aspect(
    implementation = _real_jvm_lint_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_JVM_LINT_ATTRS,
    doc = "Additive JVM lint family aspect (Checkstyle/Pmd/SpotBugs/ktlint; SpotBugs target-coupled via JavaInfo).",
)

real_jvm_format_aspect = aspect(
    implementation = _real_jvm_format_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_JVM_FORMAT_ATTRS,
    doc = "Additive JVM format family aspect (google-java-format/ktfmt).",
)

real_rust_lint_aspect = aspect(
    implementation = _real_rust_lint_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_RUST_ATTRS,
    toolchains = rust_toolchain_toolchains(),
    requires = [rust_clippy_aspect],
    doc = "Additive Rust lint family aspect (delegated Clippy).",
)

real_rust_format_aspect = aspect(
    implementation = _real_rust_format_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_RUST_ATTRS,
    toolchains = rust_toolchain_toolchains(),
    doc = "Additive Rust format family aspect (toolchain rustfmt).",
)

real_rust_typecheck_aspect = aspect(
    implementation = _real_rust_typecheck_impl,
    attr_aspects = ["aspect_hints"],
    attrs = _REAL_RUST_ATTRS,
    toolchains = rust_toolchain_toolchains(),
    doc = "Additive Rust typecheck family aspect (delegated rustc).",
)
