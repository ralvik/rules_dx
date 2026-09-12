"""Experimental minimal MDX wrappers (M21, O42).

Thin conventional boundary over the pinned `aspect_rules_js 3.4.1`
ruleset with the pinned `@mdx-js/mdx 3.1.1` compiler (see root
package.json). Each `dx_mdx_library` macro creates one private
`<name>_dx_upstream` `js_library` plus one public forwarding rule. The
forwarder preserves the upstream providers (`JsInfo`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `.mdx` srcs.

Physical/virtual ownership (framework contract): each checked-in
`.mdx` document has one physical source owner (this wrapper). ESM
import/export regions and markdown prose (including fenced code and
JSX expressions) do not become independent physical sources or core
JS/TS targets. The pinned `@mdx-js/mdx` compiler hands virtual-region
semantics to build, test, IDE, and quality integrations at execution
time; generation never compiles a document, never regex-extracts code,
and never assigns a region to the core JS/TS extensions (`.mdx` is
inert to them).

Used upstream symbols (`@aspect_rules_js//js:defs.bzl`): `js_library`;
(`@aspect_rules_js//js:providers.bzl`): `JsInfo`. No other upstream
surface is used. Consumers needing more load the upstream module
directly. MDX execution and tests reuse the JavaScript binary/test
wrappers over compiled outputs; there is no separate
`dx_mdx_binary`/`dx_mdx_test` wrapper.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"mdx": <direct .mdx>})`.
Transitive sources and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.
"""

load("@aspect_rules_js//js:defs.bzl", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

_DX_MDX_LIBRARY_PROVIDES = [
    _JsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_MDX_EXTS = [".mdx"]

def _dx_mdx_quality_sources(ctx):
    mdx = [f for f in ctx.files.srcs if f.extension == "mdx"]
    direct_sources = {}
    if len(mdx) > 0:
        direct_sources["mdx"] = depset(mdx)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _dx_mdx_preserved_providers(ctx):
    upstream = ctx.attr.upstream
    if _JsInfo not in upstream:
        fail("dx_mdx_*: upstream target has no JsInfo: " + str(ctx.attr.upstream.label))
    return [upstream[_JsInfo]]

def _dx_mdx_forwarded_runtime_providers(ctx):
    upstream = ctx.attr.upstream
    out = []
    if InstrumentedFilesInfo not in upstream:
        fail("dx_mdx_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _dx_mdx_library_forward_impl(ctx):
    return (
        _dx_mdx_preserved_providers(ctx) +
        [ctx.attr.upstream[DefaultInfo]] +
        _dx_mdx_forwarded_runtime_providers(ctx) +
        [_dx_mdx_quality_sources(ctx)]
    )

_dx_mdx_library_forward = rule(
    implementation = _dx_mdx_library_forward_impl,
    provides = _DX_MDX_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".mdx"],
            doc = "Direct MDX documents owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_JsInfo]],
            doc = "The private upstream js_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream MDX library providers unchanged and adds QualitySourcesInfo.",
)

def _dx_mdx_wrap_library(name, srcs, visibility = None, **kwargs):
    _js_library(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_mdx_library_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def dx_mdx_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for MDX documents (M21)."""
    _dx_mdx_wrap_library(name, srcs, visibility = visibility, **kwargs)
