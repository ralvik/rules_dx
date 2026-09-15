"""Experimental minimal Astro wrappers (M20, O41).

Thin conventional boundary over the pinned `aspect_rules_js 3.4.1`
ruleset with the standalone `@astrojs/compiler 4.0.0` Go+WASM compiler
(see root package.json). Each `astro_library` macro creates one
private `<name>_upstream` `js_library` plus one public forwarding
rule. The forwarder preserves the upstream providers (`JsInfo`,
`DefaultInfo`, `InstrumentedFilesInfo`) unchanged and adds
`QualitySourcesInfo` normalized from the wrapper's direct `.astro`
srcs.

Physical/virtual ownership (framework contract): each checked-in
`.astro` component has one physical source owner (this wrapper).
Frontmatter/server script, template markup, client scripts, and style
regions do not become independent physical sources or core JS/TS
targets. The pinned `@astrojs/compiler` hands virtual-region semantics
to build, test, IDE, and quality integrations at execution time;
generation never compiles a container, never regex-extracts code, and
never assigns a region to the core JS/TS extensions (`.astro` is inert
to them).

Used upstream symbols (`@aspect_rules_js//js:defs.bzl`): `js_library`;
(`@aspect_rules_js//js:providers.bzl`): `JsInfo`. No other upstream
surface is used. Consumers needing more load the upstream module
directly. Astro execution and tests reuse the JavaScript binary/test
wrappers over compiled or parsed outputs; there is no separate
`astro_binary`/`astro_test` wrapper.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"astro": <direct .astro>})`.
Transitive sources and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.
"""

load("@aspect_rules_js//js:defs.bzl", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//libs/starlark:wrapper.bzl", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_ASTRO_LIBRARY_PROVIDES = [
    _JsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_ASTRO_EXTS = [".astro"]

_astro_library_forward = dx_library_forward_rule(
    provides = _DX_ASTRO_LIBRARY_PROVIDES,
    required_providers = [(_JsInfo, "JsInfo")],
    quality_specs = [("astro", "astro")],
    what = "astro_*",
    allow_files = _ASTRO_EXTS,
    upstream_providers = [[_JsInfo]],
    doc = "Forwards upstream Astro library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Astro components owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream js_library target whose providers are preserved.",
)

def _astro_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _js_library, _astro_library_forward, srcs, visibility = visibility, **kwargs)

def astro_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for Astro components (M20)."""
    _astro_wrap_library(name, srcs, visibility = visibility, **kwargs)
