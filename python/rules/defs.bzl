"""Experimental minimal Python wrappers (M14, ADR 0010).

Thin conventional boundary over the pinned `aspect_rules_py 2.0.0-alpha.6`
ruleset. Each `dx_py_*` macro creates one private `<name>_dx_upstream`
target with the passed attributes and one public `<name>` forwarding
target. The forwarder preserves the upstream providers (`PyInfo`,
`PyWheelsInfo` for libraries, `DefaultInfo`, `InstrumentedFilesInfo`)
unchanged and adds `QualitySourcesInfo` normalized from the wrapper's
direct `srcs`. Binaries use an executable forwarder whose own symlink
action points at the upstream executable (Bazel requires
executable-providing rules to create the file themselves).

Used upstream symbols (`@aspect_rules_py//py:defs.bzl`): `py_library`,
`py_binary`, `PyInfo`, `PyWheelsInfo`. No other upstream surface is used;
consumers needing more load the upstream module directly.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"python": <direct .py>,
"python_stub": <direct .pyi>})`. Import roots, transitive sources, and
wheel closures stay readable from the preserved `PyInfo`/`PyWheelsInfo`;
no second provider duplicates them.

Python version selection follows ADR 0012 via the pinned interpreter
toolchain (`@python_interpreters//:all`, release-default 3.12). Wrappers
accept no version fields; unknown versions fail in upstream toolchain
resolution, never here.
"""

load("@aspect_rules_py//py:defs.bzl", _PyInfo = "PyInfo", _PyWheelsInfo = "PyWheelsInfo", _py_binary = "py_binary", _py_library = "py_library")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

_DX_PY_LIBRARY_PROVIDES = [
    _PyInfo,
    _PyWheelsInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_DX_PY_BINARY_PROVIDES = [
    _PyInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

def _dx_py_quality_sources(ctx):
    py = [f for f in ctx.files.srcs if f.basename.endswith(".py")]
    pyi = [f for f in ctx.files.srcs if f.basename.endswith(".pyi")]
    direct_sources = {}
    if len(py) > 0:
        direct_sources["python"] = depset(py)
    if len(pyi) > 0:
        direct_sources["python_stub"] = depset(pyi)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _dx_py_preserved_library_providers(ctx):
    upstream = ctx.attr.upstream
    if _PyInfo not in upstream:
        fail("dx_py_*: upstream target has no PyInfo: " + str(ctx.attr.upstream.label))
    if _PyWheelsInfo not in upstream:
        fail("dx_py_*: upstream target has no PyWheelsInfo: " + str(ctx.attr.upstream.label))
    return [upstream[_PyInfo], upstream[_PyWheelsInfo]]

def _dx_py_preserved_binary_providers(ctx):
    upstream = ctx.attr.upstream
    if _PyInfo not in upstream:
        fail("dx_py_*: upstream target has no PyInfo: " + str(ctx.attr.upstream.label))
    return [upstream[_PyInfo]]

def _dx_py_forwarded_runtime_providers(ctx):
    upstream = ctx.attr.upstream
    out = []
    if InstrumentedFilesInfo not in upstream:
        fail("dx_py_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _dx_py_library_forward_impl(ctx):
    return (
        _dx_py_preserved_library_providers(ctx) +
        [ctx.attr.upstream[DefaultInfo]] +
        _dx_py_forwarded_runtime_providers(ctx) +
        [_dx_py_quality_sources(ctx)]
    )

_dx_py_library_forward = rule(
    implementation = _dx_py_library_forward_impl,
    provides = _DX_PY_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".py", ".pyi"],
            doc = "Direct Python sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_PyInfo]],
            doc = "The private upstream py_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream Python library providers unchanged and adds QualitySourcesInfo.",
)

def _dx_py_symlink_default_info(ctx):
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail("dx_py_*: upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def _dx_py_forwarded_binary_non_default_providers(ctx):
    return (
        _dx_py_preserved_binary_providers(ctx) +
        _dx_py_forwarded_runtime_providers(ctx) +
        [_dx_py_quality_sources(ctx)]
    )

def _dx_py_binary_forward_impl(ctx):
    return [_dx_py_symlink_default_info(ctx)] + _dx_py_forwarded_binary_non_default_providers(ctx)

_dx_py_binary_forward = rule(
    implementation = _dx_py_binary_forward_impl,
    executable = True,
    provides = _DX_PY_BINARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".py", ".pyi"],
            doc = "Direct Python sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_PyInfo]],
            doc = "The private upstream py_binary target whose providers are preserved.",
        ),
    },
    doc = "Executable forwarder for dx_py_binary: symlinks the upstream binary.",
)

def _dx_py_wrap_library(name, srcs, visibility = None, **kwargs):
    _py_library(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_py_library_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def _dx_py_wrap_binary(name, srcs, visibility = None, **kwargs):
    _py_binary(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_py_binary_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def dx_py_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `py_library` (M14)."""
    _dx_py_wrap_library(name, srcs, visibility = visibility, **kwargs)

def dx_py_binary(name, srcs, main = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `py_binary` (M14)."""
    if main != None:
        _dx_py_wrap_binary(name, srcs, visibility = visibility, main = main, **kwargs)
    else:
        _dx_py_wrap_binary(name, srcs, visibility = visibility, **kwargs)
