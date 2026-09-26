"""Experimental minimal MDX wrappers."""

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
)

def _mdx_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _js_library, _mdx_library_forward, srcs, visibility = visibility, **kwargs)

def mdx_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over js_library for MDX documents."""
    tags = list(kwargs.pop("tags", []))
    for tag in ["no-format", "no-lint", "no-typecheck"]:
        if tag not in tags:
            tags.append(tag)
    kwargs["tags"] = tags
    _mdx_wrap_library(name, srcs, visibility = visibility, **kwargs)
