"""Experimental minimal Python wrappers (ADR 0010).

Contract: `docs/decisions/0010-python-foundation.md`, `docs/decisions/0012-language-toolchain-versions.md`.
"""

load("@aspect_rules_py//py:defs.bzl", _PyInfo = "PyInfo", _PyWheelsInfo = "PyWheelsInfo", _py_binary = "py_binary", _py_library = "py_library", _py_pytest_test = "py_pytest_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

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

_DX_PY_SOURCE_SPECS = [("python", "py"), ("python_stub", "pyi")]
_DX_PY_SOURCE_EXTS = [".py", ".pyi"]

_python_library_forward = dx_library_forward_rule(
    provides = _DX_PY_LIBRARY_PROVIDES,
    required_providers = [(_PyInfo, "PyInfo"), (_PyWheelsInfo, "PyWheelsInfo")],
    quality_specs = _DX_PY_SOURCE_SPECS,
    what = "python_*",
    allow_files = _DX_PY_SOURCE_EXTS,
    upstream_providers = [[_PyInfo]],
    doc = "Forwards upstream Python library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Python sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream py_library target whose providers are preserved.",
)

_python_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_PY_BINARY_PROVIDES,
    required_providers = [(_PyInfo, "PyInfo")],
    quality_specs = _DX_PY_SOURCE_SPECS,
    what = "python_*",
    allow_files = _DX_PY_SOURCE_EXTS,
    upstream_providers = [[_PyInfo]],
    doc = "Executable forwarder for python_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Python sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream py_binary target whose providers are preserved.",
)

_python_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_PY_BINARY_PROVIDES,
    required_providers = [(_PyInfo, "PyInfo")],
    quality_specs = _DX_PY_SOURCE_SPECS,
    what = "python_*",
    allow_files = _DX_PY_SOURCE_EXTS,
    upstream_providers = [[_PyInfo]],
    doc = "Test forwarder for python_test: symlinks the upstream pytest executable.",
    srcs_doc = "Direct Python test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream py_pytest_test target whose providers are preserved.",
    extra_attrs = {
        "_lcov_merger": attr.label(
            default = configuration_field(fragment = "coverage", name = "output_generator"),
            executable = True,
            cfg = "exec",
            doc = "Coverage-report merger. Bazel's coverage runner passes " +
                  "this magic attribute as LCOV_MERGER, which merges the " +
                  "per-test staging report into coverage.dat; without it " +
                  "the runner exits after touching an empty file even " +
                  "though the test collected coverage. Same declaration as " +
                  "upstream py_venv_exec_test.",
        ),
    },
)

def _python_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _py_library, _python_library_forward, srcs, visibility = visibility, **kwargs)

def _python_wrap_binary(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _py_binary, _python_binary_forward, srcs, visibility = visibility, **kwargs)

def python_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `py_library`."""
    _python_wrap_library(name, srcs, visibility = visibility, **kwargs)

def python_binary(name, srcs = None, main = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `py_binary`.

    Two shapes: an ordinary binary owns its `srcs` (like the handwritten
    seed), while a thin entry binary generated for a recognized
    `main.py` carries only `main` plus `deps = [":<library>"]` with no
    `srcs`. The library alone owns the source and its source-derived
    dependencies; the thin binary reports no direct sources. Both shapes
    preserve the upstream providers and execution semantics."""
    effective_srcs = srcs if srcs != None else []
    if main != None:
        _python_wrap_binary(name, effective_srcs, visibility = visibility, main = main, **kwargs)
    else:
        _python_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def python_test_rejection(kwargs):
    """Returns the contract rejection for forbidden `python_test` kwargs, or `None`.

    `python_test` always runs pytest through `py_pytest_test`, which owns
    the entrypoint wiring. Supplying a generic `main` (or any other
    alternate test driver) is rejected per the Python generation contract;
    use `py_pytest_main` plus `py_test` directly for a custom main."""
    if "main" in kwargs:
        return ("python_test always runs pytest and provides its own " +
                "entrypoint; `main` is not supported (generic mains and " +
                "alternate test drivers are rejected per " +
                "docs/testing/generation.md). Use py_pytest_main + py_test " +
                "directly for a custom main.")
    return None

def python_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `py_pytest_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. Imported non-test modules retain their ordinary
    library owners. Uses pytest and Bazel's standard test and coverage
    protocols per the Python generation contract."""
    test_srcs = srcs if srcs != None else []
    rejection = python_test_rejection(kwargs)
    if rejection != None:
        fail(rejection)
    upstream_kwargs = dict(kwargs)

    # The private upstream test stays an implementation detail via private
    # visibility; both it and the public wrapper run under `bazel test //...`
    # (no manual; double-execution is the cost of green suites).
    if "tags" in upstream_kwargs:
        kept = [t for t in upstream_kwargs["tags"] if t != "manual"]
        if len(kept) > 0:
            upstream_kwargs["tags"] = kept
        else:
            upstream_kwargs.pop("tags")
    upstream_kwargs["visibility"] = ["//visibility:private"]
    if srcs != None:
        upstream_kwargs["srcs"] = srcs
    _py_pytest_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    _python_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
        **({"aspect_hints": kwargs["aspect_hints"]} if "aspect_hints" in kwargs else {})
    )
