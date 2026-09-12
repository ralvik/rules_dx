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
`js_binary`; (`@aspect_rules_js//js:providers.bzl`): `JsInfo`. No other
upstream surface is used; consumers needing more load the upstream
module directly. Jest tests (`dx_js_test`) arrive in a later slice over
`aspect_rules_jest`; TypeScript lives under `//typescript/rules`.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"javascript": <direct .js/.jsx/.mjs/.cjs>})`.
Transitive sources and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.

Node version selection follows ADR 0012 via the pinned Node toolchain
(release-default; see MODULE.bazel). Wrappers accept no version fields;
unknown versions fail in upstream toolchain resolution, never here.
Source-only local graphs build without package-manager invocation.
"""

load("@aspect_rules_js//js:defs.bzl", _js_binary = "js_binary", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

_DX_JS_LIBRARY_PROVIDES = [
    _JsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
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
