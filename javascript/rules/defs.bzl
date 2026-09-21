"""Experimental minimal JavaScript wrappers (ADR 0013).

Contract: `docs/decisions/0013-rust-javascript-typescript-foundations.md`, `docs/decisions/0012-language-toolchain-versions.md`.
"""

load("@aspect_rules_jest//jest:defs.bzl", _jest_test = "jest_test")
load("@aspect_rules_js//js:defs.bzl", _js_binary = "js_binary", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_forward_attrs", "dx_forwarded_optional", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_quality_sources", "dx_symlink_default_info", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_JS_LIBRARY_PROVIDES = [
    _JsInfo,
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
_DX_JS_TEST_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_DX_JS_BINARY_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_JS_EXTS = [".js", ".jsx", ".mjs", ".cjs"]

_DX_JS_SOURCE_SPECS = [("javascript", ["js", "jsx", "mjs", "cjs"])]

_javascript_library_forward = dx_library_forward_rule(
    provides = _DX_JS_LIBRARY_PROVIDES,
    required_providers = [(_JsInfo, "JsInfo")],
    quality_specs = _DX_JS_SOURCE_SPECS,
    what = "javascript_*",
    allow_files = _JS_EXTS,
    upstream_providers = [[_JsInfo]],
    doc = "Forwards upstream JavaScript library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct JavaScript sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream js_library target whose providers are preserved.",
)

_javascript_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_JS_BINARY_PROVIDES,
    required_providers = [],
    quality_specs = _DX_JS_SOURCE_SPECS,
    what = "javascript_*",
    allow_files = _JS_EXTS,
    upstream_providers = [[DefaultInfo]],
    doc = "Executable forwarder for javascript_binary: symlinks the upstream binary.",
    srcs_doc = "Direct JavaScript sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream js_binary target whose providers are preserved.",
    optional_providers = [_JsInfo],
    runtime = "besteffort",
)

def _javascript_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _js_library, _javascript_library_forward, srcs, visibility = visibility, **kwargs)

def javascript_binary_upstream_data(srcs, data):
    """Computes the upstream `data` inputs for one `javascript_binary`.

    Upstream `js_binary` has no `srcs` attribute; wrapper `srcs` ride
    upstream as `data` so the `QualitySourcesInfo` owner matches the
    upstream inputs. See: `docs/quality/quality-sources.md`."""
    return list(srcs or []) + list(data or [])

def _javascript_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = dict(kwargs)
    upstream_kwargs.pop("aspect_hints", None)
    if len(srcs) > 0:
        upstream_kwargs["data"] = javascript_binary_upstream_data(srcs, upstream_kwargs.get("data"))
    _js_binary(
        name = name + "_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    _javascript_binary_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
        **({"aspect_hints": kwargs["aspect_hints"]} if "aspect_hints" in kwargs else {})
    )

def javascript_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library`."""
    _javascript_wrap_library(name, srcs, visibility = visibility, **kwargs)

def javascript_binary(name, srcs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_binary`.

    Two shapes: an ordinary binary owns its `srcs`, while a thin entry
    binary generated for a recognized entry source carries only
    `entry_point` plus `data = [":<library>"]` with no `srcs`. The
    library alone owns the source; the thin binary reports no direct
    sources. Wrapper `srcs` ride upstream as `data` (upstream has no
    `srcs`). Both shapes preserve the upstream providers and execution
    semantics. See: `docs/quality/quality-sources.md`."""
    effective_srcs = srcs if srcs != None else []
    _javascript_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def _javascript_test_forward_impl(ctx):
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
        dx_symlink_default_info(ctx, "javascript_*"),
        testing.TestEnvironment({}, env_inherit),
        dx_quality_sources(ctx.files.srcs, _DX_JS_SOURCE_SPECS, str(ctx.label)),
    ]

    # Upstream jest_test only provides InstrumentedFilesInfo when coverage
    # is enabled, so forward it conditionally (unlike the library case).
    # NB: no explicit RunEnvironmentInfo forward: constructing
    # testing.TestEnvironment above already contributes the runtime
    # environment provider, and returning both conflicts.
    return out + dx_forwarded_optional(upstream, [InstrumentedFilesInfo, OutputGroupInfo])

_javascript_test = rule(
    implementation = _javascript_test_forward_impl,
    test = True,
    provides = _DX_JS_TEST_PROVIDES,
    attrs = dx_forward_attrs(
        allow_files = _JS_EXTS,
        srcs_doc = "Direct JavaScript test sources owned by this wrapper for QualitySourcesInfo.",
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
    doc = "Test forwarder for javascript_test: symlinks the upstream jest launcher.",
)

def javascript_test_rejection(kwargs):
    """Returns the contract rejection for forbidden `javascript_test` kwargs, or `None`.

    `javascript_test` always routes through `jest_test` with the standard
    auto-configured reporters (Bazel test logs) and coverage wiring.
    Disabling the standard reporters would substitute a project-specific
    result protocol, which the JavaScript generation contract forbids."""
    if kwargs.get("auto_configure_reporters", True) == False:
        return ("javascript_test always uses jest with the standard " +
                "auto-configured reporters (Bazel test logs); " +
                "`auto_configure_reporters = False` is not supported " +
                "(project-specific result protocols are rejected per " +
                "docs/testing/generation.md).")
    return None

def javascript_test_env(env_inherit):
    """Computes the effective test-runtime inherited environment.

    Mirrors the caller's list and always adds `TESTBRIDGE_TEST_ONLY`, which
    is the Bazel test-filtering (`--test_filter`/sharding) channel the
    upstream launcher only receives through `TestEnvironment`. The
    forwarder rebuilds that provider from this exact list, so filtering
    support is structural, never caller-dependent."""
    env = list(env_inherit) if env_inherit != None else []
    if "TESTBRIDGE_TEST_ONLY" not in env:
        env.append("TESTBRIDGE_TEST_ONLY")
    return env

def javascript_test(name, srcs, node_modules, data = None, visibility = None, tags = None, env_inherit = None, **kwargs):
    """Experimental minimal wrapper over `jest_test`.

    The private `<name>_upstream` target runs the full jest graph
    (`srcs` plus caller `data`, with `jest-cli`/`jest-junit` linked from
    `node_modules` by the upstream macro). The public `<name>` test
    target symlinks the upstream launcher and preserves
    `testing.TestEnvironment` (reporter/test-filter wiring) plus the
    runtime providers, adding `QualitySourcesInfo` normalized from the
    wrapper's direct `srcs`.

    Jest executes tests as CommonJS by default: first-party `.js` is ESM
    (the root package.json sets `"type": "module"`), so tests covering
    ESM sources need
    `node_options = ["--experimental-vm-modules"]` (see
    `//javascript/tests/fixtures/hello:hello_test`) until the transform slice wires
    static ESM/TS support. The macro always adds `//:package_json` to
    the upstream data: jest detects ESM by walking up the runfiles tree
    from each test file, so the scope file must be a runtime input of
    every test. Upstream-owned runfiles are unaffected (npm packages
    carry their own package.json, generated helpers are `.cjs`/`.mjs`)."""
    upstream_data = list(srcs) + (list(data) if data != None else [])
    effective_env = javascript_test_env(env_inherit)
    rejection = javascript_test_rejection(kwargs)
    if rejection != None:
        fail(rejection)

    # The private upstream test stays an implementation detail via private
    # visibility; both it and the public wrapper run under `bazel test //...`
    # (no manual; double-execution is the cost of green suites).
    # `aspect_hints` rides the public forwarder only (quality aspects visit
    # the forwarder); strip it from the upstream jest_test kwargs.
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

    # Workspace ESM scope marker (see docstring): must resolve in runfiles
    # above every first-party test source. Referenced as the root
    # js_library: js rules reject cross-package source files in data.
    if "//:package_json" not in upstream_data:
        upstream_data.append("//:package_json")
    _jest_test(
        name = name + "_upstream",
        node_modules = node_modules,
        data = upstream_data,
        env_inherit = effective_env,
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    _javascript_test(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        env_inherit = effective_env,
        visibility = visibility,
        tags = [t for t in tags if t != "manual"] if tags != None else None,
        **({"aspect_hints": kwargs["aspect_hints"]} if "aspect_hints" in kwargs else {})
    )
