"""Experimental minimal TypeScript wrappers (M16, ADR 0013).

Thin conventional boundary over the pinned `aspect_rules_ts 3.10.0`
ruleset with the TypeScript 5.9.3 toolchain (see MODULE.bazel). Each
`typescript_*` macro creates one private `<name>_upstream` target with the
passed attributes and one public `<name>` forwarding target. The forwarder
preserves the upstream providers (`JsInfo`, `TsConfigInfo`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `srcs`.

Used upstream symbols (`@aspect_rules_ts//ts:defs.bzl`): `ts_project`,
`TsConfigInfo`; (`@aspect_rules_js//js:providers.bzl`): `JsInfo`. No other
upstream surface is used; consumers needing more load the upstream
module directly. TypeScript execution reuses the JavaScript binary/test
wrappers (`//javascript/rules`) over compiled outputs; there is no
separate `typescript_binary`/`typescript_test` wrapper.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"typescript": <direct .ts/.mts/.cts>,
"tsx": <direct .tsx>})`. Declaration files (`.d.ts`, `.d.mts`, `.d.cts`)
are inert per the generation contract and must not be passed as `srcs`.
Transitive sources/types and npm closures stay readable from the preserved
`JsInfo`; no second provider duplicates them.

TypeScript version selection follows ADR 0012 via the pinned toolchain
(`@npm_typescript`, release-default 5.9.3). Wrappers accept no version
fields; unknown versions fail in upstream toolchain resolution, never here.
Source-only local graphs build without package-manager invocation.
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

def _typescript_wrap_project(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _ts_project, _typescript_project_forward, srcs, visibility = visibility, **kwargs)

def typescript_project(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `ts_project` (M16)."""
    _typescript_wrap_project(name, srcs, visibility = visibility, **kwargs)
