"""Experimental minimal MDX wrappers (M21, O42).

Thin conventional boundary over the pinned `aspect_rules_js 3.4.1`
ruleset with the pinned `@mdx-js/mdx 3.1.1` compiler (see root
package.json). Each `mdx_library` macro creates one private
`<name>_upstream` `js_library` plus one public forwarding rule. The
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
`mdx_binary`/`mdx_test` wrapper.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"mdx": <direct .mdx>})`.
Transitive sources and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.
"""

load("@aspect_rules_js//js:defs.bzl", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//libs/starlark:wrapper.bzl", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_MDX_LIBRARY_PROVIDES = [
    _JsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_MDX_EXTS = [".mdx"]

_mdx_library_forward = dx_library_forward_rule(
    provides = _DX_MDX_LIBRARY_PROVIDES,
    required_providers = [(_JsInfo, "JsInfo")],
    quality_specs = [("mdx", "mdx")],
    what = "mdx_*",
    allow_files = _MDX_EXTS,
    upstream_providers = [[_JsInfo]],
    doc = "Forwards upstream MDX library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct MDX documents owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream js_library target whose providers are preserved.",
)

def _mdx_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _js_library, _mdx_library_forward, srcs, visibility = visibility, **kwargs)

def mdx_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for MDX documents (M21)."""
    _mdx_wrap_library(name, srcs, visibility = visibility, **kwargs)
