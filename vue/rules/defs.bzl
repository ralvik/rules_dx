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
)

def _vue_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _js_library, _vue_library_forward, srcs, visibility = visibility, **kwargs)

def vue_library(name, srcs, visibility = None, **kwargs):

    tags = list(kwargs.pop("tags", []))
    for tag in ["no-format", "no-lint", "no-typecheck"]:
        if tag not in tags:
            tags.append(tag)
    kwargs["tags"] = tags
    _vue_wrap_library(name, srcs, visibility = visibility, **kwargs)
