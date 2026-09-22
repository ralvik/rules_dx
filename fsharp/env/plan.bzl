"""Focused F# environment plan (WP2).

Contract: `docs/environments/fsharp.md`.
"""

load("//env:focused.bzl", "focused_direct_sources", "focused_simple_plan", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//quality:sources.bzl", "QualitySourcesInfo")

FSharpEnvPlanInfo = provider(
    doc = "Provider-derived focused F# target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct F# sources.",
        "has_sources": "Whether the wrapper owns any direct sources.",
        "source_count": "Number of direct sources.",
        "target": "Display label of the planned wrapper target.",
    },
)

def _fsharp_env_plan_impl(ctx):
    target = ctx.attr.target
    if QualitySourcesInfo not in target:
        fail("fsharp_env_plan: target has no QualitySourcesInfo: " + display_label(target.label))
    direct = focused_direct_sources(target)
    info = focused_simple_plan(direct, display_label(ctx.attr.target.label))
    plan = info.plan
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        FSharpEnvPlanInfo(
            direct_sources = direct,
            has_sources = info.has_sources,
            source_count = info.source_count,
            target = plan["target"],
        ),
        DxSubjectInfo(fields = plan),
    ]

fsharp_env_plan = rule(
    implementation = _fsharp_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One fsharp_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused F# environment plan for one wrapper target (WP2).",
)
