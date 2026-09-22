"""Focused C/C++ environment plan (WP2).

Contract: `docs/environments/cc.md`.
"""

load("//env:focused.bzl", "focused_direct_sources", "focused_simple_plan", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//quality:sources.bzl", "QualitySourcesInfo")

CcEnvPlanInfo = provider(
    doc = "Provider-derived focused C/C++ target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct C/C++ sources.",
        "has_sources": "Whether the wrapper owns any direct C/C++ sources.",
        "source_count": "Number of direct C/C++ sources.",
        "target": "Display label of the planned wrapper target.",
    },
)

def _cc_env_plan_impl(ctx):
    target = ctx.attr.target
    if QualitySourcesInfo not in target:
        fail("cc_env_plan: target has no QualitySourcesInfo: " + display_label(target.label))
    direct = focused_direct_sources(target)
    info = focused_simple_plan(direct, display_label(ctx.attr.target.label))
    plan = info.plan
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        CcEnvPlanInfo(
            direct_sources = direct,
            has_sources = info.has_sources,
            source_count = info.source_count,
            target = plan["target"],
        ),
        DxSubjectInfo(fields = plan),
    ]

cc_env_plan = rule(
    implementation = _cc_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            doc = "One cc_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused C/C++ environment plan for one wrapper target (WP2).",
)
