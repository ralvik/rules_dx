"""Experimental minimal Vue wrappers (M18, O29).

Thin conventional boundary over the pinned `aspect_rules_js 3.4.1`
ruleset with the Vue 3.5.42 runtime/compiler (see root package.json).
Each `vue_library` macro creates one private `<name>_upstream`
`js_library` plus one public forwarding rule. The forwarder preserves
the upstream providers (`JsInfo`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `.vue` srcs.

Physical/virtual ownership (framework contract): each checked-in `.vue`
single-file component has one physical source owner (this wrapper).
Embedded script, template, and style regions do not become independent
physical sources or core JS/TS targets. The pinned
`@vue/compiler-sfc` hands virtual-region semantics to build, test, IDE,
and quality integrations at execution time; generation never compiles a
container, never regex-extracts code, and never assigns a region to the
core JS/TS extensions (`.vue` is inert to them).

Used upstream symbols (`@aspect_rules_js//js:defs.bzl`): `js_library`;
(`@aspect_rules_js//js:providers.bzl`): `JsInfo`. No other upstream
surface is used. Consumers needing more load the upstream module
directly. Vue execution and tests reuse the JavaScript binary/test
wrappers over compiled or parsed outputs; there is no separate
`vue_binary`/`vue_test` wrapper.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"vue": <direct .vue>})`.
Transitive sources and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.
"""

load("@aspect_rules_js//js:defs.bzl", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

_DX_VUE_LIBRARY_PROVIDES = [
    _JsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_VUE_EXTS = [".vue"]

def _vue_quality_sources(ctx):
    vue = [f for f in ctx.files.srcs if f.extension == "vue"]
    direct_sources = {}
    if len(vue) > 0:
        direct_sources["vue"] = depset(vue)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _vue_preserved_providers(ctx):
    upstream = ctx.attr.upstream
    if _JsInfo not in upstream:
        fail("vue_*: upstream target has no JsInfo: " + str(ctx.attr.upstream.label))
    return [upstream[_JsInfo]]

def _vue_forwarded_runtime_providers(ctx):
    upstream = ctx.attr.upstream
    out = []
    if InstrumentedFilesInfo not in upstream:
        fail("vue_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _vue_library_forward_impl(ctx):
    return (
        _vue_preserved_providers(ctx) +
        [ctx.attr.upstream[DefaultInfo]] +
        _vue_forwarded_runtime_providers(ctx) +
        [_vue_quality_sources(ctx)]
    )

_vue_library_forward = rule(
    implementation = _vue_library_forward_impl,
    provides = _DX_VUE_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = _VUE_EXTS,
            doc = "Direct Vue single-file components owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_JsInfo]],
            doc = "The private upstream js_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream Vue library providers unchanged and adds QualitySourcesInfo.",
)

def _vue_wrap_library(name, srcs, visibility = None, **kwargs):
    _js_library(
        name = name + "_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _vue_library_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def vue_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for Vue SFCs (M18)."""
    _vue_wrap_library(name, srcs, visibility = visibility, **kwargs)
