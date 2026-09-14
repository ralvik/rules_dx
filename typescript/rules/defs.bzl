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
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

_DX_TS_PROJECT_PROVIDES = [
    _JsInfo,
    _TsConfigInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

def _typescript_is_declaration(basename):
    return basename.endswith(".d.ts") or basename.endswith(".d.mts") or basename.endswith(".d.cts")

def _typescript_quality_sources(ctx):
    ts = [
        f
        for f in ctx.files.srcs
        if (f.extension in ["ts", "mts", "cts"] and not _typescript_is_declaration(f.basename))
    ]
    tsx = [f for f in ctx.files.srcs if f.extension == "tsx"]
    direct_sources = {}
    if len(ts) > 0:
        direct_sources["typescript"] = depset(ts)
    if len(tsx) > 0:
        direct_sources["tsx"] = depset(tsx)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _typescript_preserved_providers(ctx):
    upstream = ctx.attr.upstream
    if _JsInfo not in upstream:
        fail("typescript_*: upstream target has no JsInfo: " + str(ctx.attr.upstream.label))
    if _TsConfigInfo not in upstream:
        fail("typescript_*: upstream target has no TsConfigInfo: " + str(ctx.attr.upstream.label))
    return [upstream[_JsInfo], upstream[_TsConfigInfo]]

def _typescript_forwarded_runtime_providers(ctx):
    upstream = ctx.attr.upstream
    out = []
    if InstrumentedFilesInfo not in upstream:
        fail("typescript_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _typescript_project_forward_impl(ctx):
    return (
        _typescript_preserved_providers(ctx) +
        [ctx.attr.upstream[DefaultInfo]] +
        _typescript_forwarded_runtime_providers(ctx) +
        [_typescript_quality_sources(ctx)]
    )

_typescript_project_forward = rule(
    implementation = _typescript_project_forward_impl,
    provides = _DX_TS_PROJECT_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".ts", ".tsx", ".mts", ".cts"],
            doc = "Direct TypeScript sources owned by this wrapper for QualitySourcesInfo. Declaration files (.d.ts/.d.mts/.d.cts) are inert and must not be listed.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_JsInfo]],
            doc = "The private upstream ts_project target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream TypeScript project providers unchanged and adds QualitySourcesInfo.",
)

def _typescript_wrap_project(name, srcs, visibility = None, **kwargs):
    _ts_project(
        name = name + "_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _typescript_project_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def typescript_project(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `ts_project` (M16)."""
    _typescript_wrap_project(name, srcs, visibility = visibility, **kwargs)
