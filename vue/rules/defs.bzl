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
load("//libs/starlark:wrapper.bzl", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_VUE_LIBRARY_PROVIDES = [
    _JsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_VUE_EXTS = [".vue"]

_vue_library_forward = dx_library_forward_rule(
    provides = _DX_VUE_LIBRARY_PROVIDES,
    required_providers = [(_JsInfo, "JsInfo")],
    quality_specs = [("vue", "vue")],
    what = "vue_*",
    allow_files = _VUE_EXTS,
    upstream_providers = [[_JsInfo]],
    doc = "Forwards upstream Vue library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Vue single-file components owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream js_library target whose providers are preserved.",
)

def _vue_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _js_library, _vue_library_forward, srcs, visibility = visibility, **kwargs)

def vue_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for Vue SFCs (M18)."""
    _vue_wrap_library(name, srcs, visibility = visibility, **kwargs)
