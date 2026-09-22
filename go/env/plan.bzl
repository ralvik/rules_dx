"""Focused Go environment plan (WP2).

Contract: `docs/environments/go.md`.
"""

load("@rules_go//go:def.bzl", _GoArchive = "GoArchive")
load("//env:focused.bzl", "focused_closure_plan", "focused_direct_sources", "focused_go_transitive", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//quality:sources.bzl", "QualitySourcesInfo")

GoEnvPlanInfo = provider(
    doc = "Provider-derived focused Go target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct Go sources.",
        "has_sources": "Whether the wrapper owns any direct Go sources.",
        "has_tests": "Whether direct plus transitive closure carries test sources.",
        "source_count": "Number of direct Go sources.",
        "target": "Display label of the planned wrapper target.",
        "test_source_count": "Number of test sources in direct plus transitive closure.",
        "test_sources": "Sorted basenames of test sources in direct plus transitive closure.",
        "transitive_source_count": "Number of files in the transitive GoArchive source closure.",
        "transitive_sources": "Sorted basenames of the transitive GoArchive source closure.",
    },
)

def _go_env_plan_impl(ctx):
    target = ctx.attr.target
    if QualitySourcesInfo not in target:
        fail("go_env_plan: target has no QualitySourcesInfo: " + display_label(target.label))
    if _GoArchive not in target:
        fail("go_env_plan: target has no GoArchive: " + display_label(target.label))
    direct = focused_direct_sources(target)
    transitive = focused_go_transitive(target[_GoArchive].transitive)
    info = focused_closure_plan(direct, transitive, display_label(ctx.attr.target.label))
    plan = info.plan
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        GoEnvPlanInfo(
            direct_sources = direct,
            has_sources = info.has_sources,
            has_tests = info.has_tests,
            source_count = info.source_count,
            target = plan["target"],
            test_source_count = info.test_source_count,
            test_sources = info.test_sources,
            transitive_source_count = info.transitive_source_count,
            transitive_sources = info.transitive_sources,
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
