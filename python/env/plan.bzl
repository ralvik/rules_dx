"""Focused Python environment plan (M14 WP3).

`PythonEnvPlanInfo` is the provider-derived focused-target plan contribution:
direct sources, configured import roots, transitive first-party sources, and
wheel-closure size read from the analyzed authoritative providers of one
`dx_py_*` wrapper target. It never scans the checkout, never re-resolves uv
metadata, and never mutates environment or codegen selection.

`python_env_plan` materializes that plan as a deterministic JSON file plus a
`DxSubjectInfo` observation surface for `starlark_test` analysis mode.
Wheel size comes from `dx_python_env_wheels_aspect` (see `aspect.bzl`),
which merges `PyWheelsInfo` along the existing wrapper/upstream/venv/deps
edges; imports and sources come from the wrapper's preserved providers.
Repository/root/exact-target orchestration, collection, atomic selection,
and `dx env`/`dx setup` remain M25.
"""

load("@aspect_rules_py//py:defs.bzl", _PyInfo = "PyInfo")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//python/env:aspect.bzl", "PythonEnvWheelsInfo", "dx_python_env_wheels_aspect")
load("//quality:sources.bzl", "QualitySourcesInfo")

PythonEnvPlanInfo = provider(
    doc = "Provider-derived focused Python target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct Python sources.",
        "has_wheels": "Whether the transitive wheel closure is non-empty.",
        "imports": "Sorted configured import roots from authoritative PyInfo.",
        "target": "Display label of the planned wrapper target.",
        "transitive_sources": "Sorted basenames of transitive first-party sources.",
        "wheel_count": "Number of wheels in the transitive PyWheelsInfo closure.",
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

def _python_env_plan_impl(ctx):
    target = ctx.attr.target
    if _PyInfo not in target:
        fail("python_env_plan: target has no PyInfo: " + display_label(target.label))
    py_info = target[_PyInfo]
    imports = sorted(py_info.imports.to_list())
    # First-party/driver `.py` closure for import projection. Wheel payloads
    # are covered by `wheel_count` plus the site-packages import roots above;
    # raw venv markers (e.g. repeated `actual_install.install`) carry no
    # import identity and would collapse lossily under basename projection.
    seen = {}
    for f in py_info.transitive_sources.to_list():
        if f.basename.endswith(".py"):
            seen[f.basename] = True
    transitive = sorted(seen.keys())
    direct = _direct_sources(target)
    if PythonEnvWheelsInfo not in target:
        fail("python_env_plan: wheels aspect missing on target: " + display_label(target.label))
    wheel_count = len(target[PythonEnvWheelsInfo].wheels.to_list())
    has_wheels = wheel_count > 0
    plan = {
        "direct_sources": ",".join(direct),
        "has_wheels": str(has_wheels),
        "imports": ",".join(imports),
        "target": display_label(ctx.attr.target.label),
        "transitive_sources": ",".join(transitive),
        "wheel_count": str(wheel_count),
    }
    out = ctx.actions.declare_file(ctx.label.name + ".json")
    ctx.actions.write(out, json.encode(plan) + "\n")
    return [
        DefaultInfo(files = depset([out])),
        PythonEnvPlanInfo(
            direct_sources = direct,
            has_wheels = has_wheels,
            imports = imports,
            target = plan["target"],
            transitive_sources = transitive,
            wheel_count = wheel_count,
        ),
        DxSubjectInfo(fields = plan),
    ]

python_env_plan = rule(
    implementation = _python_env_plan_impl,
    attrs = {
        "target": attr.label(
            mandatory = True,
            aspects = [dx_python_env_wheels_aspect],
            doc = "One dx_py_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused Python environment plan for one wrapper target (M14 WP3).",
)
