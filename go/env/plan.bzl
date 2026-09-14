"""Focused Go environment plan (M22 WP2).

`GoEnvPlanInfo` is the provider-derived focused-target plan contribution:
direct sources, source count, and target identity read from the analyzed
authoritative providers of one `go_*` wrapper target. It never scans the
checkout, never re-resolves module metadata, and never mutates environment
or codegen selection.

`go_env_plan` materializes that plan as a deterministic JSON file plus a
`DxSubjectInfo` observation surface for `starlark_test` analysis mode.
Direct sources come from `QualitySourcesInfo`; the Go toolchain closure
stays readable from the preserved `GoInfo`/`GoArchive` without duplicating
it here. Repository/root/exact-target orchestration, collection, atomic
selection, and `dx env`/`dx setup` remain M25.
"""

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

def _direct_sources(target):
    if QualitySourcesInfo not in target:
        return []
    info = target[QualitySourcesInfo]
    out = []
    for class_id in sorted(info.direct_sources.keys()):
        for f in info.direct_sources[class_id].to_list():
            out.append(f.basename)
    return sorted(out)

def _go_env_plan_impl(ctx):
    target = ctx.attr.target
    if QualitySourcesInfo not in target:
        fail("go_env_plan: target has no QualitySourcesInfo: " + display_label(target.label))
    direct = _direct_sources(target)
    source_count = len(direct)
    has_sources = source_count > 0
    plan = {
        "direct_sources": ",".join(direct),
        "has_sources": str(has_sources),
        "source_count": str(source_count),
        "target": display_label(ctx.attr.target.label),
    }
    out = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(out, json.encode(plan) + "\n")
    return [
        DefaultInfo(files = depset([out])),
        GoEnvPlanInfo(
            direct_sources = direct,
            has_sources = has_sources,
            source_count = source_count,
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
    doc = "Emits the provider-derived focused Go environment plan for one wrapper target (M22 WP2).",
)
