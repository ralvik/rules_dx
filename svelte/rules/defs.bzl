"""Experimental minimal Svelte wrappers.

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
    """Experimental minimal wrapper over `js_library` for Svelte components."""

    tags = list(kwargs.pop("tags", []))
    for tag in ["no-format", "no-lint", "no-typecheck"]:
        if tag not in tags:
            tags.append(tag)
    kwargs["tags"] = tags
    _svelte_wrap_library(name, srcs, visibility = visibility, **kwargs)
