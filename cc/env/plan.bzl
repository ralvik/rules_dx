"""Focused C/C++ environment plan (WP2).

Contract: `docs/environments/environment.md`.
"""

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

def _direct_sources(target):
    if QualitySourcesInfo not in target:
        return []
    info = target[QualitySourcesInfo]
    out = []
    for class_id in sorted(info.direct_sources.keys()):
        for f in info.direct_sources[class_id].to_list():
            out.append(f.basename)
    return sorted(out)

def _cc_env_plan_impl(ctx):
    target = ctx.attr.target
    if QualitySourcesInfo not in target:
        fail("cc_env_plan: target has no QualitySourcesInfo: " + display_label(target.label))
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
        CcEnvPlanInfo(
            direct_sources = direct,
            has_sources = has_sources,
            source_count = source_count,
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
