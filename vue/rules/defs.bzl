"""Experimental minimal Vue wrappers.

Contract: `libs/starlark/wrapper.bzl`.
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
    # Lane-A: aspect_hints ride the public forwarder via dx_wrap.
    dx_wrap(name, _js_library, _vue_library_forward, srcs, visibility = visibility, **kwargs)

def vue_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `js_library` for Vue SFCs."""

    # PARITY_DEFERRED (ADR 0019): no adapter claims `vue` yet. Tag the
    # public forwarder so the fail-closed deferred pipeline does not fail
    # analysis for every consumer of this wrapper; remove when an adapter
    # claims the class (See: quality/parity_tests.bzl).
    tags = list(kwargs.pop("tags", []))
    for tag in ["no-format", "no-lint", "no-typecheck"]:
        if tag not in tags:
            tags.append(tag)
    kwargs["tags"] = tags
    _vue_wrap_library(name, srcs, visibility = visibility, **kwargs)
