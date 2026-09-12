"""Experimental minimal Astro wrappers (M20, O41).

Thin conventional boundary over the pinned `aspect_rules_js 3.4.1`
ruleset with the standalone `@astrojs/compiler 4.0.0` Go+WASM compiler
(see root package.json). Each `dx_astro_library` macro creates one
private `<name>_dx_upstream` `js_library` plus one public forwarding
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
`dx_astro_binary`/`dx_astro_test` wrapper.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"astro": <direct .astro>})`.
Transitive sources and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.
"""

load("@aspect_rules_js//js:defs.bzl", _js_library = "js_library")
load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

_DX_ASTRO_LIBRARY_PROVIDES = [
    _JsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_ASTRO_EXTS = [".astro"]

def _dx_astro_quality_sources(ctx):
    astro = [f for f in ctx.files.srcs if f.extension == "astro"]
    direct_sources = {}
    if len(astro) > 0:
        direct_sources["astro"] = depset(astro)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _dx_astro_preserved_providers(ctx):
    upstream = ctx.attr.upstream
    if _JsInfo not in upstream:
        fail("dx_astro_*: upstream target has no JsInfo: " + str(ctx.attr.upstream.label))
    return [upstream[_JsInfo]]

def _dx_astro_forwarded_runtime_providers(ctx):
    upstream = ctx.attr.upstream
    out = []
    if InstrumentedFilesInfo not in upstream:
        fail("dx_astro_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _dx_astro_library_forward_impl(ctx):
    return (
        _dx_astro_preserved_providers(ctx) +
        [ctx.attr.upstream[DefaultInfo]] +
        _dx_astro_forwarded_runtime_providers(ctx) +
        [_dx_astro_quality_sources(ctx)]
    )

_dx_astro_library_forward = rule(
    implementation = _dx_astro_library_forward_impl,
    provides = _DX_ASTRO_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".astro"],
            doc = "Direct Astro components owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_JsInfo]],
            doc = "The private upstream js_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream Astro library providers unchanged and adds QualitySourcesInfo.",
)

def _dx_astro_wrap_library(name, srcs, visibility = None, **kwargs):
    _js_library(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_astro_library_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def dx_astro_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for Astro components (M20)."""
    _dx_astro_wrap_library(name, srcs, visibility = visibility, **kwargs)
