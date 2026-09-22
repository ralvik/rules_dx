"""Experimental minimal TypeScript wrappers (ADR 0013).

Contract: `docs/decisions/0013-rust-javascript-typescript-foundations.md`, `docs/decisions/0012-language-toolchain-versions.md`.
"""

load("@aspect_rules_jest//jest:defs.bzl", _jest_test = "jest_test")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("@aspect_rules_ts//ts:defs.bzl", _TsConfigInfo = "TsConfigInfo", _ts_project = "ts_project")
load("//libs/starlark:wrapper.bzl", "dx_forward_attrs", "dx_forwarded_optional", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_quality_sources", "dx_symlink_default_info", "dx_test_forward_kwargs", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_TS_PROJECT_PROVIDES = [
    _JsInfo,
    _TsConfigInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# NB: testing.TestEnvironment is returned by the test forwarder (the test
# runner reads it from the target) but cannot be listed here: it is a
# constructor value, not a Provider object. InstrumentedFilesInfo is
# likewise returned only when coverage is enabled (matching upstream
# jest_test) and so cannot be advertised unconditionally; coverage still
# works because the runner reads it from the target, not via provides.
_DX_TS_TEST_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

# Declaration files (`.d.ts`, `.d.mts`, `.d.cts`) are inert per the
# generation contract and must not be passed as `srcs`; the exclusion
# suffixes below keep them out of `QualitySourcesInfo` even if listed.
_DX_TS_SOURCE_SPECS = [
    ("typescript", ["ts", "mts", "cts"], [".d.ts", ".d.mts", ".d.cts"]),
    ("tsx", "tsx"),
]
_DX_TS_SOURCE_EXTS = [".ts", ".tsx", ".mts", ".cts"]

_typescript_project_forward = dx_library_forward_rule(
    provides = _DX_TS_PROJECT_PROVIDES,
    required_providers = [(_JsInfo, "JsInfo"), (_TsConfigInfo, "TsConfigInfo")],
    quality_specs = _DX_TS_SOURCE_SPECS,
    what = "typescript_*",
    allow_files = _DX_TS_SOURCE_EXTS,
    upstream_providers = [[_JsInfo]],
    doc = "Forwards upstream TypeScript project providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct TypeScript sources owned by this wrapper for QualitySourcesInfo. Declaration files (.d.ts/.d.mts/.d.cts) are inert and must not be listed.",
    upstream_doc = "The private upstream ts_project target whose providers are preserved.",
)

_DX_TS_DECLARATION_SUFFIXES = [".d.ts", ".d.mts", ".d.cts"]

def _is_declaration(src):
    """Returns whether a source path is an inert declaration file."""
    for suffix in _DX_TS_DECLARATION_SUFFIXES:
        if src.endswith(suffix):
            return True
    return False

def typescript_srcs_rejection(srcs):
    """Returns the contract rejection for forbidden `typescript_project` srcs, or `None`.

    Declaration files (`.d.ts`, `.d.mts`, `.d.cts`) are inert per the
    generation contract and must not be passed as `srcs`: `tsc` inputs
    and configuration come from `typescript_project` over real sources,
    and no wrapper independently enumerates sources or invokes a second
    compiler. Silently dropping them from `QualitySourcesInfo` would mask
    the authoring error, so they fail here instead."""
    bad = [src for src in srcs or [] if _is_declaration(src)]
    if bad:
        return ("typescript_project takes real sources only; declaration " +
                "files are inert and must not be listed in srcs " +
                "(rejected per docs/testing/generation.md): " +
                ", ".join(sorted(bad)))
    return None

def _typescript_wrap_project(name, srcs, visibility = None, **kwargs):
    rejection = typescript_srcs_rejection(srcs)
    if rejection != None:
        fail(rejection)
    # Lane-A: aspect_hints ride the public forwarder via dx_wrap.
    dx_wrap(name, _ts_project, _typescript_project_forward, srcs, visibility = visibility, **kwargs)

def typescript_project(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `ts_project`."""
    _typescript_wrap_project(name, srcs, visibility = visibility, **kwargs)

def _typescript_test_forward_impl(ctx):
    upstream = ctx.attr.upstream

    # The upstream launcher already bakes fixed_env (JEST_JUNIT_OUTPUT_FILE,
    # snapshot flags) into its executable, which we symlink with runfiles
    # merged below. Only env_inherit (notably TESTBRIDGE_TEST_ONLY for
    # sharding/--test_filter) lives solely in TestEnvironment, so rebuild
    # it from the mirrored env_inherit attribute.
    env_inherit = list(ctx.attr.env_inherit) if ctx.attr.env_inherit else []
    if "TESTBRIDGE_TEST_ONLY" not in env_inherit:
        env_inherit.append("TESTBRIDGE_TEST_ONLY")
    out = [
        dx_symlink_default_info(ctx, "typescript_*"),
        testing.TestEnvironment({}, env_inherit),
        dx_quality_sources(ctx.files.srcs, _DX_TS_SOURCE_SPECS, str(ctx.label)),
    ]

    # Upstream jest_test only provides InstrumentedFilesInfo when coverage
    # is enabled, so forward it conditionally (unlike the library case).
    # NB: no explicit RunEnvironmentInfo forward: constructing
    # testing.TestEnvironment above already contributes the runtime
    # environment provider, and returning both conflicts.
    return out + dx_forwarded_optional(upstream, [InstrumentedFilesInfo, OutputGroupInfo], "typescript_*")

_typescript_test = rule(
    implementation = _typescript_test_forward_impl,
    test = True,
    provides = _DX_TS_TEST_PROVIDES,
    attrs = dx_forward_attrs(
        allow_files = _DX_TS_SOURCE_EXTS,
        srcs_doc = "Direct TypeScript test sources owned by this wrapper for QualitySourcesInfo.",
        upstream_providers = [[DefaultInfo]],
        upstream_doc = "The private upstream jest_test target whose providers are preserved.",
        extra_attrs = {
            "env_inherit": attr.string_list(
                doc = "Environment variables to inherit at test runtime, " +
                      "mirrored from the upstream jest_test (TESTBRIDGE_TEST_ONLY " +
                      "is always added for sharding/--test_filter).",
            ),
        } | dx_lcov_merger_attr(),
    ),
    doc = "Test forwarder for typescript_test: symlinks the upstream jest launcher.",
)

def typescript_test_rejection(kwargs):
    """Returns the contract rejection for forbidden `typescript_test` kwargs, or `None`.

    See: `docs/testing/generation.md`."""
    if kwargs.get("auto_configure_reporters", True) == False:
        return ("typescript_test always uses jest with the standard " +
                "auto-configured reporters (Bazel test logs); " +
                "`auto_configure_reporters = False` is not supported " +
                "(project-specific result protocols are rejected per " +
                "docs/testing/generation.md).")
    return None

def typescript_test_env(env_inherit):
    """Computes the effective test-runtime inherited environment.

    See: `docs/testing/generation.md`."""
    env = list(env_inherit) if env_inherit != None else []
    if "TESTBRIDGE_TEST_ONLY" not in env:
        env.append("TESTBRIDGE_TEST_ONLY")
    return env

def typescript_test(name, srcs, node_modules, data = None, deps = None, tsconfig = None, transpiler = None, declaration = None, visibility = None, tags = None, env_inherit = None, **kwargs):
    """Experimental minimal wrapper over `jest_test` for TypeScript sources.

    The private `<name>_ts` target compiles the TypeScript test `srcs`
    with `ts_project` (using `deps` plus `tsconfig`/`transpiler`/
    `declaration`); the private `<name>_upstream` target runs the full
    jest graph over the compiled test plus caller `deps`/`data`, with
    `jest-cli`/`jest-junit` linked from `node_modules` by the upstream
    macro. The public `<name>` test target symlinks the upstream launcher
    and preserves `testing.TestEnvironment` (reporter/test-filter wiring)
    plus the runtime providers, adding `QualitySourcesInfo` normalized
    from the wrapper's direct TypeScript `srcs`.

    Additional `tsc` options belong in the `tsconfig` file, not wrapper
    attrs. Execution of TypeScript entries reuses `javascript_binary`
    over the compiled output; there is no `typescript_binary`.

    See: `docs/decisions/0013-rust-javascript-typescript-foundations.md`."""

    # Declaration sources are inert for the library wrapper and for tests.
    rejection = typescript_srcs_rejection(srcs)
    if rejection != None:
        fail(rejection)
    reporter_rejection = typescript_test_rejection(kwargs)
    if reporter_rejection != None:
        fail(reporter_rejection)
    effective_env = typescript_test_env(env_inherit)

    # Private tsc compilation of the test sources. `aspect_hints` rides
    # the public forwarder only (quality aspects visit the forwarder);
    # strip it here. `tags` stay test-only (jest upstream plus forwarder).
    ts_kwargs = {}
    if deps != None:
        ts_kwargs["deps"] = list(deps)
    if tsconfig != None:
        ts_kwargs["tsconfig"] = tsconfig
    if transpiler != None:
        ts_kwargs["transpiler"] = transpiler
    if declaration != None:
        ts_kwargs["declaration"] = declaration
    _ts_project(
        name = name + "_ts",
        srcs = srcs,
        testonly = True,
        visibility = ["//visibility:private"],
        **ts_kwargs
    )

    # Jest runs the compiled test plus the compiled libraries. The
    # compiled `<name>_ts` target already carries its `deps` closure, but
    # list `deps` explicitly as well so handwritten `data`-only callers
    # keep working and the runtime edge stays obvious.
    upstream_data = [":" + name + "_ts"] + list(deps or []) + list(data or [])
    if "//:package_json" not in upstream_data:
        upstream_data.append("//:package_json")

    upstream_kwargs = dict(kwargs)
    upstream_kwargs.pop("aspect_hints", None)
    if tags != None:
        kept = [t for t in tags if t != "manual"]
        if len(kept) > 0:
            upstream_kwargs["tags"] = kept
        elif "tags" in upstream_kwargs:
            upstream_kwargs.pop("tags")
    elif "tags" in upstream_kwargs:
        upstream_kwargs.pop("tags")

    _jest_test(
        name = name + "_upstream",
        node_modules = node_modules,
        data = upstream_data,
        env_inherit = effective_env,
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    forward_kwargs = dx_test_forward_kwargs(kwargs)
    _typescript_test(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        env_inherit = effective_env,
        visibility = visibility,
        tags = [t for t in tags if t != "manual"] if tags != None else forward_kwargs.pop("tags", None),
        **forward_kwargs
    )
