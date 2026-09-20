"""Experimental minimal TypeScript wrappers (ADR 0013).

Contract: `docs/decisions/0013-rust-javascript-typescript-foundations.md`, `docs/decisions/0012-language-toolchain-versions.md`.
"""

load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("@aspect_rules_ts//ts:defs.bzl", _TsConfigInfo = "TsConfigInfo", _ts_project = "ts_project")
load("//libs/starlark:wrapper.bzl", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_TS_PROJECT_PROVIDES = [
    _JsInfo,
    _TsConfigInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# Declaration files (`.d.ts`, `.d.mts`, `.d.cts`) are inert per the
# generation contract and must not be passed as `srcs`; the exclusion
# suffixes below keep them out of `QualitySourcesInfo` even if listed.
_DX_TS_SOURCE_SPECS = [
    ("typescript", ["ts", "mts", "cts"], [".d.ts", ".d.mts", ".d.cts"]),
    ("tsx", "tsx"),
]
_DX_TS_SOURCE_EXTS = [".ts", ".tsx", ".mts", ".cts"]

_typescript_project_forward = dx_library_forward_rule(
    provides = _DX_TS_PROJECT_PROVIDES,
    required_providers = [(_JsInfo, "JsInfo"), (_TsConfigInfo, "TsConfigInfo")],
    quality_specs = _DX_TS_SOURCE_SPECS,
    what = "typescript_*",
    allow_files = _DX_TS_SOURCE_EXTS,
    upstream_providers = [[_JsInfo]],
    doc = "Forwards upstream TypeScript project providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct TypeScript sources owned by this wrapper for QualitySourcesInfo. Declaration files (.d.ts/.d.mts/.d.cts) are inert and must not be listed.",
    upstream_doc = "The private upstream ts_project target whose providers are preserved.",
)

_DX_TS_DECLARATION_SUFFIXES = [".d.ts", ".d.mts", ".d.cts"]

def _is_declaration(src):
    """Returns whether a source path is an inert declaration file."""
    for suffix in _DX_TS_DECLARATION_SUFFIXES:
        if src.endswith(suffix):
            return True
    return False

def typescript_srcs_rejection(srcs):
    """Returns the contract rejection for forbidden `typescript_project` srcs, or `None`.

    Declaration files (`.d.ts`, `.d.mts`, `.d.cts`) are inert per the
    generation contract and must not be passed as `srcs`: `tsc` inputs
    and configuration come from `typescript_project` over real sources,
    and no wrapper independently enumerates sources or invokes a second
    compiler. Silently dropping them from `QualitySourcesInfo` would mask
    the authoring error, so they fail here instead."""
    bad = [src for src in srcs or [] if _is_declaration(src)]
    if bad:
        return ("typescript_project takes real sources only; declaration " +
                "files are inert and must not be listed in srcs " +
                "(rejected per docs/testing/generation.md): " +
                ", ".join(sorted(bad)))
    return None

def _typescript_wrap_project(name, srcs, visibility = None, **kwargs):
    rejection = typescript_srcs_rejection(srcs)
    if rejection != None:
        fail(rejection)
    dx_wrap(name, _ts_project, _typescript_project_forward, srcs, visibility = visibility, **kwargs)

def typescript_project(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `ts_project`."""
    _typescript_wrap_project(name, srcs, visibility = visibility, **kwargs)
