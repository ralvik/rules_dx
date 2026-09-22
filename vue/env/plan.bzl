"""Focused Vue environment plan (WP2).

Contract: `docs/environments/vue.md`.
"""

load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//env:focused.bzl", "focused_direct_sources", "focused_js_closure", "focused_js_plan", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

VueEnvPlanInfo = provider(
    doc = "Provider-derived focused Vue target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct Vue sources.",
        "has_npm": "Whether the transitive npm closure is non-empty.",
        "npm_source_count": "Number of files in the transitive JsInfo npm_sources closure.",
        "target": "Display label of the planned wrapper target.",
        "transitive_sources": "Sorted basenames of transitive JsInfo sources.",
    },
)

def _vue_env_plan_impl(ctx):
    target = ctx.attr.target
    if _JsInfo not in target:
        fail("vue_env_plan: target has no JsInfo: " + display_label(target.label))
    js_info = target[_JsInfo]
    closure = focused_js_closure(js_info.transitive_sources.to_list(), js_info.npm_sources.to_list())
    direct = focused_direct_sources(target)
    plan = focused_js_plan(direct, closure, display_label(ctx.attr.target.label))
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        VueEnvPlanInfo(
            direct_sources = direct,
            has_npm = closure.has_npm,
            npm_source_count = closure.npm_count,
            target = plan["target"],
            transitive_sources = closure.transitive,
        ),
        DxSubjectInfo(fields = plan),
    ]

vue_env_plan = rule(
    implementation = _vue_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One vue_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused Vue environment plan for one wrapper target (WP2).",
)
