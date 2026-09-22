"""Focused TypeScript environment plan (WP3).

Contract: `docs/environments/environment.md`.
"""

load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("@aspect_rules_ts//ts:defs.bzl", _TsConfigInfo = "TsConfigInfo")
load("//env:focused.bzl", "focused_direct_sources", "focused_js_closure", "focused_typescript_plan", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

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

def _typescript_env_plan_impl(ctx):
    target = ctx.attr.target
    if _JsInfo not in target:
        fail("typescript_env_plan: target has no JsInfo: " + display_label(target.label))
    js_info = target[_JsInfo]
    closure = focused_js_closure(js_info.transitive_sources.to_list(), js_info.npm_sources.to_list())
    direct = focused_direct_sources(target)
    has_tsconfig = _TsConfigInfo in target
    plan = focused_typescript_plan(direct, closure, has_tsconfig, display_label(ctx.attr.target.label))
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        TypeScriptEnvPlanInfo(
            direct_sources = direct,
            has_npm = closure.has_npm,
            has_tsconfig = has_tsconfig,
            npm_source_count = closure.npm_count,
            target = plan["target"],
            transitive_sources = closure.transitive,
        ),
        DxSubjectInfo(fields = plan),
    ]

typescript_env_plan = rule(
    implementation = _typescript_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One typescript_project wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused TypeScript environment plan for one wrapper target (WP3).",
)
