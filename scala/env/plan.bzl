load("@rules_java//java:defs.bzl", "JavaInfo")
load("//env:focused.bzl", "focused_closure_plan", "focused_direct_sources", "focused_transitive_basenames", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//quality:sources.bzl", "QualitySourcesInfo")

ScalaEnvPlanInfo = provider(
    fields = {
        "direct_sources": "Sorted basenames of direct Scala/Java sources.",
        "has_sources": "Whether the wrapper owns any direct sources.",
        "has_tests": "Whether direct plus transitive closure carries test sources.",
        "source_count": "Number of direct sources.",
        "target": "Display label of the planned wrapper target.",
        "test_source_count": "Number of test sources in direct plus transitive closure.",
        "test_sources": "Sorted basenames of test sources in direct plus transitive closure.",
        "transitive_source_count": "Number of files in the transitive JavaInfo source-jar closure.",
        "transitive_sources": "Sorted basenames of the transitive JavaInfo source-jar closure.",
    },
)

def _scala_env_plan_impl(ctx):
    target = ctx.attr.target
    if QualitySourcesInfo not in target:
        fail("scala_env_plan: target has no QualitySourcesInfo: " + display_label(target.label))
    if JavaInfo not in target:
        fail("scala_env_plan: target has no JavaInfo: " + display_label(target.label))
    direct = focused_direct_sources(target)
    transitive = focused_transitive_basenames(target[JavaInfo].transitive_source_jars)
    info = focused_closure_plan(direct, transitive, display_label(ctx.attr.target.label))
    plan = info.plan
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        ScalaEnvPlanInfo(
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

scala_env_plan = rule(
    implementation = _scala_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
        ),
    },
)
