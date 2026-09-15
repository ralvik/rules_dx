"""Experimental minimal Svelte wrappers (M19, O40).

Thin conventional boundary over the pinned `aspect_rules_js 3.4.1`
ruleset with the Svelte 5.57.0 runtime/compiler (see root package.json).
Each `svelte_library` macro creates one private `<name>_upstream`
`js_library` plus one public forwarding rule. The forwarder preserves
the upstream providers (`JsInfo`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `.svelte` srcs.

Physical/virtual ownership (framework contract): each checked-in `.svelte`
component has one physical source owner (this wrapper). Embedded script
(instance and module scripts), template markup, and style regions do not
become independent physical sources or core JS/TS targets. The pinned
`svelte/compiler` hands virtual-region semantics to build, test, IDE,
and quality integrations at execution time; generation never compiles a
container, never regex-extracts code, and never assigns a region to the
core JS/TS extensions (`.svelte` is inert to them).

Used upstream symbols (`@aspect_rules_js//js:defs.bzl`): `js_library`;
(`@aspect_rules_js//js:providers.bzl`): `JsInfo`. No other upstream
surface is used. Consumers needing more load the upstream module
directly. Svelte execution and tests reuse the JavaScript binary/test
wrappers over compiled or parsed outputs; there is no separate
`svelte_binary`/`svelte_test` wrapper.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"svelte": <direct .svelte>})`.
Transitive sources and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.
"""

load("@aspect_rules_js//js:defs.bzl", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//libs/starlark:wrapper.bzl", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_SVELTE_LIBRARY_PROVIDES = [
    _JsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_SVELTE_EXTS = [".svelte"]

_svelte_library_forward = dx_library_forward_rule(
    provides = _DX_SVELTE_LIBRARY_PROVIDES,
    required_providers = [(_JsInfo, "JsInfo")],
    quality_specs = [("svelte", "svelte")],
    what = "svelte_*",
    allow_files = _SVELTE_EXTS,
    upstream_providers = [[_JsInfo]],
    doc = "Forwards upstream Svelte library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Svelte components owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream js_library target whose providers are preserved.",
)

def _svelte_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _js_library, _svelte_library_forward, srcs, visibility = visibility, **kwargs)

def svelte_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for Svelte components (M19)."""
    _svelte_wrap_library(name, srcs, visibility = visibility, **kwargs)
