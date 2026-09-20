"""Experimental minimal MDX wrappers (M21, O42).

Contract: `libs/starlark/wrapper.bzl`.
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
