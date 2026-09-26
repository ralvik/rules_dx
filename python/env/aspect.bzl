
load("@aspect_rules_py//py:defs.bzl", _PyWheelsInfo = "PyWheelsInfo")

PythonEnvWheelsInfo = provider(
    fields = {
        "wheels": "Postorder depset of wheel record structs, merged like upstream py_library aggregation.",
    },
)

_ASPECT_ATTRS = ["upstream", "venv", "deps"]

def _edge_targets(attrs, name):
    value = getattr(attrs, name, [])
    if value == None:
        return []
    if type(value) == "Target":
        return [value]
    return value

def _dx_python_env_wheels_aspect_impl(target, ctx):
    transitive = []
    if _PyWheelsInfo in target:
        transitive.append(target[_PyWheelsInfo].wheels)
    for name in _ASPECT_ATTRS:
        for dep in _edge_targets(ctx.rule.attr, name):
            if PythonEnvWheelsInfo in dep:
                transitive.append(dep[PythonEnvWheelsInfo].wheels)
    return [PythonEnvWheelsInfo(wheels = depset(order = "postorder", transitive = transitive))]

dx_python_env_wheels_aspect = aspect(
    implementation = _dx_python_env_wheels_aspect_impl,
    attr_aspects = _ASPECT_ATTRS,
)
