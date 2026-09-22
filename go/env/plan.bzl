"""Focused Go environment plan (WP2).

Contract: `docs/environments/go.md`.
"""

load("//env:focused.bzl", "focused_direct_sources", "focused_simple_plan", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//quality:sources.bzl", "QualitySourcesInfo")

GoEnvPlanInfo = provider(
    doc = "Provider-derived focused Go target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct Go sources.",
        "has_sources": "Whether the wrapper owns any direct Go sources.",
        "source_count": "Number of direct Go sources.",
        "target": "Display label of the planned wrapper target.",
    },
)

def _go_env_plan_impl(ctx):
    target = ctx.attr.target
    if QualitySourcesInfo not in target:
        fail("go_env_plan: target has no QualitySourcesInfo: " + display_label(target.label))
    direct = focused_direct_sources(target)
    info = focused_simple_plan(direct, display_label(ctx.attr.target.label))
    plan = info.plan
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        GoEnvPlanInfo(
            direct_sources = direct,
            has_sources = info.has_sources,
            source_count = info.source_count,
            target = plan["target"],
        ),
        DxSubjectInfo(fields = plan),
    ]

go_env_plan = rule(
    implementation = _go_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One go_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused Go environment plan for one wrapper target (WP2).",
)
