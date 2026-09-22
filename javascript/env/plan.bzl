"""Focused JavaScript environment plan (WP3).

Contract: `docs/environments/environment.md`.
"""

load("@aspect_rules_js//js:providers.bzl", _JsInfo = "JsInfo")
load("//env:focused.bzl", "focused_direct_sources", "focused_js_closure", "focused_js_plan", "focused_npm_store_projection", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")

JavaScriptEnvPlanInfo = provider(
    doc = "Provider-derived focused JavaScript target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct JavaScript sources.",
        "has_npm": "Whether the transitive npm closure is non-empty.",
        "has_store": "Whether the pnpm store closure is non-empty.",
        "npm_source_count": "Number of files in the transitive JsInfo npm_sources closure.",
        "store_count": "Number of entries in the JsInfo npm_package_store_infos closure.",
        "target": "Display label of the planned wrapper target.",
        "transitive_sources": "Sorted basenames of transitive JsInfo sources.",
    },
)

def _javascript_env_plan_impl(ctx):
    target = ctx.attr.target
    if _JsInfo not in target:
        fail("javascript_env_plan: target has no JsInfo: " + display_label(target.label))
    js_info = target[_JsInfo]
    closure = focused_js_closure(js_info.transitive_sources.to_list(), js_info.npm_sources.to_list())
    direct = focused_direct_sources(target)
    plan = focused_js_plan(direct, closure, display_label(ctx.attr.target.label))
    store = focused_npm_store_projection(js_info.npm_package_store_infos.to_list())
    plan["has_store"] = str(store.has_store)
    plan["store_count"] = str(store.store_count)
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        JavaScriptEnvPlanInfo(
            direct_sources = direct,
            has_npm = closure.has_npm,
            has_store = store.has_store,
            npm_source_count = closure.npm_count,
            store_count = store.store_count,
            target = plan["target"],
            transitive_sources = closure.transitive,
        ),
        DxSubjectInfo(fields = plan),
    ]

javascript_env_plan = rule(
    implementation = _javascript_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One javascript_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused JavaScript environment plan for one wrapper target (WP3).",
)
