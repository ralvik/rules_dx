"""Focused JavaScript environment plan (M16 WP3).

`JavaScriptEnvPlanInfo` is the provider-derived focused-target plan
contribution: direct sources, transitive first-party sources, and npm-closure
size read from the analyzed authoritative providers of one `dx_js_*`
wrapper target. It never scans the checkout, never invokes pnpm, never
re-resolves package metadata, and never mutates environment or codegen
selection.

`javascript_env_plan` materializes that plan as a deterministic JSON file
plus a `DxSubjectInfo` observation surface for `starlark_test` analysis
mode. Sources come from the wrapper's preserved `JsInfo`; direct sources
come from `QualitySourcesInfo`. Importer-local `node_modules` facades and
repository/root/exact-target orchestration remain M25.
"""

load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//quality:sources.bzl", "QualitySourcesInfo")

JavaScriptEnvPlanInfo = provider(
    doc = "Provider-derived focused JavaScript target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct JavaScript sources.",
        "has_npm": "Whether the transitive npm closure is non-empty.",
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

def _javascript_env_plan_impl(ctx):
    target = ctx.attr.target
    if _JsInfo not in target:
        fail("javascript_env_plan: target has no JsInfo: " + display_label(target.label))
    js_info = target[_JsInfo]
    seen = {}
    for f in js_info.transitive_sources.to_list():
        seen[f.basename] = True
    transitive = sorted(seen.keys())
    direct = _direct_sources(target)
    npm_sources = js_info.npm_sources.to_list()
    npm_source_count = len(npm_sources)
    has_npm = npm_source_count > 0
    plan = {
        "direct_sources": ",".join(direct),
        "has_npm": str(has_npm),
        "npm_source_count": str(npm_source_count),
        "target": display_label(ctx.attr.target.label),
        "transitive_sources": ",".join(transitive),
    }
    out = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(out, json.encode(plan) + "\n")
    return [
        DefaultInfo(files = depset([out])),
        JavaScriptEnvPlanInfo(
            direct_sources = direct,
            has_npm = has_npm,
            npm_source_count = npm_source_count,
            target = plan["target"],
            transitive_sources = transitive,
        ),
        DxSubjectInfo(fields = plan),
    ]

javascript_env_plan = rule(
    implementation = _javascript_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One dx_js_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused JavaScript environment plan for one wrapper target (M16 WP3).",
)
