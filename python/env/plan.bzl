"""Focused Python environment plan (WP3).

"""

load("@aspect_rules_py//py:defs.bzl", _PyInfo = "PyInfo")
load("//env:focused.bzl", "focused_direct_sources", "focused_python_plan", "focused_python_transitive", "focused_venv_projection", "focused_write_plan")
load("//libs/starlark:defs.bzl", "DxSubjectInfo", "display_label")
load("//python/env:aspect.bzl", "PythonEnvWheelsInfo", "dx_python_env_wheels_aspect")

PythonEnvPlanInfo = provider(
    doc = "Provider-derived focused Python target environment plan.",
    fields = {
        "direct_sources": "Sorted basenames of direct Python sources.",
        "has_venv": "Whether RunEnvironmentInfo projects a materialized .venv.",
        "has_wheels": "Whether the transitive wheel closure is non-empty.",
        "imports": "Sorted configured import roots from authoritative PyInfo.",
        "target": "Display label of the planned wrapper target.",
        "transitive_sources": "Sorted basenames of transitive first-party sources.",
        "venv": "Workspace-relative VIRTUAL_ENV path, else empty.",
        "wheel_count": "Number of wheels in the transitive PyWheelsInfo closure.",
    },
)

def _python_env_plan_impl(ctx):
    target = ctx.attr.target
    if _PyInfo not in target:
        fail("python_env_plan: target has no PyInfo: " + display_label(target.label))
    py_info = target[_PyInfo]
    imports = sorted(py_info.imports.to_list())
    transitive = focused_python_transitive(py_info.transitive_sources.to_list())
    direct = focused_direct_sources(target)
    if PythonEnvWheelsInfo not in target:
        fail("python_env_plan: wheels aspect missing on target: " + display_label(target.label))
    wheel_count = len(target[PythonEnvWheelsInfo].wheels.to_list())
    info = focused_python_plan(direct, transitive, imports, wheel_count, display_label(ctx.attr.target.label))
    plan = info.plan
    venv = focused_venv_projection(target)
    plan["has_venv"] = str(venv.has_venv)
    plan["venv"] = venv.venv
    out = focused_write_plan(ctx, plan)
    return [
        DefaultInfo(files = depset([out])),
        PythonEnvPlanInfo(
            direct_sources = direct,
            has_venv = venv.has_venv,
            has_wheels = info.has_wheels,
            imports = imports,
            target = plan["target"],
            transitive_sources = transitive,
            venv = venv.venv,
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
            doc = "One python_* wrapper target to plan (focused target only).",
        ),
    },
    doc = "Emits the provider-derived focused Python environment plan for one wrapper target (WP3).",
)
