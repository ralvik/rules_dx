"""Focused TypeScript environment plan (M16 WP3).

`TypeScriptEnvPlanInfo` is the provider-derived focused-target plan
contribution: direct sources, transitive compiled sources, npm-closure size,
and tsconfig presence read from the analyzed authoritative providers of one
`dx_ts_project` wrapper target. It never scans the checkout, never invokes
pnpm, never re-resolves package metadata, never runs `tsc`, and never
mutates environment or codegen selection.

`typescript_env_plan` materializes that plan as a deterministic JSON file
plus a `DxSubjectInfo` observation surface for `starlark_test` analysis
mode. Sources come from the wrapper's preserved `JsInfo`; direct sources
come from `QualitySourcesInfo`; `TsConfigInfo` presence is recorded but its
contents are never interpreted here. Importer-local `node_modules` facades
and repository/root/exact-target orchestration remain M25.
"""

load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("@aspect_rules_ts//ts:defs.bzl", _TsConfigInfo = "TsConfigInfo")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//quality:sources.bzl", "QualitySourcesInfo")

TypeScriptEnvPlanInfo = provider(
    doc = "Provider-derived focused TypeScript target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct TypeScript sources.",
        "has_npm": "Whether the transitive npm closure is non-empty.",
        "has_tsconfig": "Whether the target preserves TsConfigInfo.",
        "npm_source_count": "Number of files in the transitive JsInfo npm_sources closure.",
        "target": "Display label of the planned wrapper target.",
        "transitive_sources": "Sorted basenames of transitive JsInfo sources.",
    },
)

def _direct_sources(target):
    if QualitySourcesInfo not in target:
        return []
    info = target[QualitySourcesInfo]
    out = []
    for class_id in sorted(info.direct_sources.keys()):
        for f in info.direct_sources[class_id].to_list():
            out.append(f.basename)
    return sorted(out)

def _typescript_env_plan_impl(ctx):
    target = ctx.attr.target
    if _JsInfo not in target:
        fail("typescript_env_plan: target has no JsInfo: " + display_label(target.label))
    js_info = target[_JsInfo]
    seen = {}
    for f in js_info.transitive_sources.to_list():
        seen[f.basename] = True
    transitive = sorted(seen.keys())
    direct = _direct_sources(target)
    npm_sources = js_info.npm_sources.to_list()
    npm_source_count = len(npm_sources)
    has_npm = npm_source_count > 0
    has_tsconfig = _TsConfigInfo in target
    plan = {
        "direct_sources": ",".join(direct),
        "has_npm": str(has_npm),
        "has_tsconfig": str(has_tsconfig),
        "npm_source_count": str(npm_source_count),
        "target": display_label(ctx.attr.target.label),
        "transitive_sources": ",".join(transitive),
    }
    out = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(out, json.encode(plan) + "\n")
    return [
        DefaultInfo(files = depset([out])),
        TypeScriptEnvPlanInfo(
            direct_sources = direct,
            has_npm = has_npm,
            has_tsconfig = has_tsconfig,
            npm_source_count = npm_source_count,
            target = plan["target"],
            transitive_sources = transitive,
        ),
        DxSubjectInfo(fields = plan),
    ]

typescript_env_plan = rule(
    implementation = _typescript_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One dx_ts_project wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused TypeScript environment plan for one wrapper target (M16 WP3).",
)
