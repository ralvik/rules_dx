"""Wheel-closure aspect for focused Python environment plans (M14 WP3).

Upstream exec targets (`py_binary` / `py_pytest_test`) surface the sibling
venv's imports and sources via `PyInfo` but carry no `PyWheelsInfo` — the
sibling venv is a terminal artifact that consumes wheel records at
assembly without re-emitting them. The records themselves live on the
`py_library`-shaped dep closure (`PyWheelsInfo.wheels`, aggregated
postorder).

`dx_python_env_wheels_aspect` walks the existing edges from a `dx_py_*`
wrapper — `upstream` into the private upstream target, `venv` into the
sibling venv lib, `deps` into the library-shaped closure — and merges
each node's `PyWheelsInfo.wheels` with upstream's postorder merge. It
never scans the checkout, never re-resolves uv metadata, and adds no
dependency edges: every traversed edge already exists in the analyzed
graph, so no toolchain transition or visibility change is involved.
Focused-target only.
"""

load("@aspect_rules_py//py:defs.bzl", _PyWheelsInfo = "PyWheelsInfo")

PythonEnvWheelsInfo = provider(
    doc = "Merged wheel records in the focused target's closure.",
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
    doc = "Merges PyWheelsInfo.wheels along the wrapper/upstream/venv/deps edges for one focused Python target (M14 WP3).",
)
