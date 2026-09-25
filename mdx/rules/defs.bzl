"""Experimental minimal MDX wrappers.

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
    # Lane-A: aspect_hints ride the public forwarder via dx_wrap.
    dx_wrap(name, _js_library, _mdx_library_forward, srcs, visibility = visibility, **kwargs)

def mdx_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for MDX documents."""

    # PARITY_DEFERRED (ADR 0019): no adapter claims `mdx` yet. Tag the
    # public forwarder so the fail-closed deferred pipeline does not fail
    # analysis for every consumer of this wrapper; remove when an adapter
    # claims the class (See: quality/parity_tests.bzl).
    tags = list(kwargs.pop("tags", []))
    for tag in ["no-format", "no-lint", "no-typecheck"]:
        if tag not in tags:
            tags.append(tag)
    kwargs["tags"] = tags
    _mdx_wrap_library(name, srcs, visibility = visibility, **kwargs)
