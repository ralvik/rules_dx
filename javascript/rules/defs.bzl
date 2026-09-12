"""Experimental minimal JavaScript wrappers (M16, ADR 0013).

Thin conventional boundary over the pinned `aspect_rules_js 3.4.1`
ruleset. Each `dx_js_*` macro creates one private `<name>_dx_upstream`
target with the passed attributes and one public `<name>` forwarding
target. The forwarder preserves the upstream providers (`JsInfo` for
libraries, `DefaultInfo`, `InstrumentedFilesInfo`) unchanged and adds
`QualitySourcesInfo` normalized from the wrapper's direct `srcs`.
Binaries use an executable forwarder whose own symlink action points at
the upstream executable (Bazel requires executable-providing rules to
create the file themselves).

Used upstream symbols (`@aspect_rules_js//js:defs.bzl`): `js_library`,
`js_binary`; (`@aspect_rules_js//js:providers.bzl`): `JsInfo`;
(`@aspect_rules_jest//jest:defs.bzl`): `jest_test`. No other upstream
surface is used; consumers needing more load the upstream module
directly. TypeScript lives under `//typescript/rules`.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"javascript": <direct .js/.jsx/.mjs/.cjs>})`.
Transitive sources and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.

Node version selection follows ADR 0012 via the pinned Node toolchain
(release-default; see MODULE.bazel). Wrappers accept no version fields;
unknown versions fail in upstream toolchain resolution, never here.
Source-only local graphs build without package-manager invocation.
"""

load("@aspect_rules_jest//jest:defs.bzl", _jest_test = "jest_test")
load("@aspect_rules_js//js:defs.bzl", _js_binary = "js_binary", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

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

def _dx_js_quality_sources(ctx):
    js = [f for f in ctx.files.srcs if "." + f.extension in _JS_EXTS or f.basename.endswith(".js")]
    direct_sources = {}
    if len(js) > 0:
        direct_sources["javascript"] = depset(js)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _dx_js_preserved_providers(ctx):
    upstream = ctx.attr.upstream
    if _JsInfo not in upstream:
        fail("dx_js_*: upstream target has no JsInfo: " + str(ctx.attr.upstream.label))
    return [upstream[_JsInfo]]

def _dx_js_forwarded_runtime_providers(ctx):
    upstream = ctx.attr.upstream
    out = []
    if InstrumentedFilesInfo not in upstream:
        fail("dx_js_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _dx_js_library_forward_impl(ctx):
    return (
        _dx_js_preserved_providers(ctx) +
        [ctx.attr.upstream[DefaultInfo]] +
        _dx_js_forwarded_runtime_providers(ctx) +
        [_dx_js_quality_sources(ctx)]
    )

_dx_js_library_forward = rule(
    implementation = _dx_js_library_forward_impl,
    provides = _DX_JS_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".js", ".jsx", ".mjs", ".cjs"],
            doc = "Direct JavaScript sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_JsInfo]],
            doc = "The private upstream js_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream JavaScript library providers unchanged and adds QualitySourcesInfo.",
)

def _dx_js_symlink_default_info(ctx):
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail("dx_js_*: upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def _dx_js_forwarded_binary_non_default_providers(ctx):
    upstream = ctx.attr.upstream
    out = []
    if _JsInfo in upstream:
        out.append(upstream[_JsInfo])
    if InstrumentedFilesInfo in upstream:
        out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out + [_dx_js_quality_sources(ctx)]

def _dx_js_binary_forward_impl(ctx):
    return [_dx_js_symlink_default_info(ctx)] + _dx_js_forwarded_binary_non_default_providers(ctx)

_dx_js_binary_forward = rule(
    implementation = _dx_js_binary_forward_impl,
    executable = True,
    provides = _DX_JS_BINARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".js", ".jsx", ".mjs", ".cjs"],
            doc = "Direct JavaScript sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[DefaultInfo]],
            doc = "The private upstream js_binary target whose providers are preserved.",
        ),
    },
    doc = "Executable forwarder for dx_js_binary: symlinks the upstream binary.",
)

def _dx_js_wrap_library(name, srcs, visibility = None, **kwargs):
    _js_library(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_js_library_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def _dx_js_wrap_binary(name, srcs, visibility = None, **kwargs):
    _js_binary(
        name = name + "_dx_upstream",
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_js_binary_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def dx_js_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` (M16)."""
    _dx_js_wrap_library(name, srcs, visibility = visibility, **kwargs)

def dx_js_binary(name, srcs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_binary` (M16).

    Two shapes: an ordinary binary owns its `srcs`, while a thin entry
    binary generated for a recognized entry source carries only
    `entry_point` plus `data = [":<library>"]` with no `srcs`. The
    library alone owns the source; the thin binary reports no direct
    sources. Both shapes preserve the upstream providers and execution
    semantics.

    Args:
      name: public binary target name (upstream target is name_dx_upstream).
      srcs: direct binary sources for QualitySourcesInfo; empty for thin entries.
      visibility: visibility of the public forwarding binary target.
      **kwargs: extra attributes forwarded to the upstream js_binary
        (entry_point, data, etc.).
    """
    effective_srcs = srcs if srcs != None else []
    _dx_js_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def _dx_js_test_forward_impl(ctx):
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
        _dx_js_symlink_default_info(ctx),
        testing.TestEnvironment({}, env_inherit),
        _dx_js_quality_sources(ctx),
    ]

    # Upstream jest_test only provides InstrumentedFilesInfo when coverage
    # is enabled, so forward it conditionally (unlike the library case).
    if InstrumentedFilesInfo in upstream:
        out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])

    # NB: no explicit RunEnvironmentInfo forward: constructing
    # testing.TestEnvironment above already contributes the runtime
    # environment provider, and returning both conflicts.
    return out

_dx_js_test = rule(
    implementation = _dx_js_test_forward_impl,
    test = True,
    provides = _DX_JS_TEST_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".js", ".jsx", ".mjs", ".cjs"],
            doc = "Direct JavaScript test sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[DefaultInfo]],
            doc = "The private upstream jest_test target whose providers are preserved.",
        ),
        "env_inherit": attr.string_list(
            doc = "Environment variables to inherit at test runtime, " +
                  "mirrored from the upstream jest_test (TESTBRIDGE_TEST_ONLY " +
                  "is always added for sharding/--test_filter).",
        ),
        "_lcov_merger": attr.label(
            default = configuration_field(fragment = "coverage", name = "output_generator"),
            executable = True,
            cfg = "exec",
            doc = "Coverage-report merger. Bazel's coverage runner passes " +
                  "this magic attribute as LCOV_MERGER, which merges the " +
                  "per-test staging report into coverage.dat; without it " +
                  "the runner exits after touching an empty file even " +
                  "though the test collected coverage. Same declaration as " +
                  "upstream jest_test.",
        ),
    },
    doc = "Test forwarder for dx_js_test: symlinks the upstream jest launcher.",
)

def dx_js_test(name, srcs, node_modules, data = None, visibility = None, tags = None, env_inherit = None, **kwargs):
    """Experimental minimal wrapper over `jest_test` (M16).

    The private `<name>_dx_upstream` target runs the full jest graph
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
    `//javascript/hello:hello_test`) until the transform slice wires
    static ESM/TS support. The macro always adds `//:package_json` to
    the upstream data: jest detects ESM by walking up the runfiles tree
    from each test file, so the scope file must be a runtime input of
    every test. Upstream-owned runfiles are unaffected (npm packages
    carry their own package.json, generated helpers are `.cjs`/`.mjs`).

    Args:
      name: public test target name (upstream target is name_dx_upstream).
      srcs: direct test sources owned by this wrapper.
      node_modules: label of the linked node_modules target (e.g.
        `//:node_modules`) where `jest-cli` (and `jest-junit` when
        reporters stay auto-configured) is linked.
      data: extra runtime deps (files under test, configs); `srcs`
        are always included.
      visibility: visibility of the public forwarding test target.
      tags: extra tags for both targets; the upstream target is
        additionally `manual` so `bazel test //...` exercises the
        public wrapper only.
      env_inherit: extra runtime-inherited env vars, mirrored to both
        the upstream jest_test and the rebuilt TestEnvironment.
      **kwargs: extra attributes forwarded to the upstream jest_test
        (config, snapshots, size, timeout, etc.).
    """
    upstream_data = list(srcs) + (list(data) if data != None else [])

    # Workspace ESM scope marker (see docstring): must resolve in runfiles
    # above every first-party test source. Referenced as the root
    # js_library: js rules reject cross-package source files in data.
    if "//:package_json" not in upstream_data:
        upstream_data.append("//:package_json")
    _jest_test(
        name = name + "_dx_upstream",
        node_modules = node_modules,
        data = upstream_data,
        env_inherit = env_inherit,
        visibility = ["//visibility:private"],
        tags = (list(tags) if tags != None else []) + ["manual"],
        **kwargs
    )
    _dx_js_test(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        env_inherit = env_inherit,
        visibility = visibility,
        tags = tags,
    )
