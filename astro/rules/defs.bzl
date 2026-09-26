"""Experimental minimal Astro wrappers."""

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
)

def _astro_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _js_library, _astro_library_forward, srcs, visibility = visibility, **kwargs)

def astro_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over js_library for Astro components."""
    tags = list(kwargs.pop("tags", []))
    for tag in ["no-format", "no-lint", "no-typecheck"]:
        if tag not in tags:
            tags.append(tag)
    kwargs["tags"] = tags
    _astro_wrap_library(name, srcs, visibility = visibility, **kwargs)
