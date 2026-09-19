"""Experimental minimal Python wrappers (M14, ADR 0010).

Thin conventional boundary over the pinned `aspect_rules_py 2.0.0-alpha.6`
ruleset. Each `python_*` macro creates one private `<name>_upstream`
target with the passed attributes and one public `<name>` forwarding
target. The forwarder preserves the upstream providers (`PyInfo`,
`PyWheelsInfo` for libraries, `DefaultInfo`, `InstrumentedFilesInfo`)
unchanged and adds `QualitySourcesInfo` normalized from the wrapper's
direct `srcs`. Binaries use an executable forwarder whose own symlink
action points at the upstream executable (Bazel requires
executable-providing rules to create the file themselves).

Used upstream symbols (`@aspect_rules_py//py:defs.bzl`): `py_library`,
`py_binary`, `py_pytest_test`, `PyInfo`, `PyWheelsInfo`. No other upstream
surface is used; consumers needing more load the upstream module directly.

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
    """Experimental minimal wrapper over `py_library` (M14)."""
    _python_wrap_library(name, srcs, visibility = visibility, **kwargs)

def python_binary(name, srcs = None, main = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `py_binary` (M14).

    Two shapes: an ordinary binary owns its `srcs` (like the handwritten
    seed), while a thin entry binary generated for a recognized
    `main.py` carries only `main` plus `deps = [":<library>"]` with no
    `srcs`. The library alone owns the source and its source-derived
    dependencies; the thin binary reports no direct sources. Both shapes
    preserve the upstream providers and execution semantics.

    Args:
      name: public binary target name (upstream target is name_upstream).
      srcs: direct binary sources; empty for thin entry binaries.
      main: entry source for thin binaries; none for ordinary binaries.
      visibility: visibility of the public forwarding binary target.
      **kwargs: extra attributes forwarded to the upstream py_binary.
    """
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
    use `py_pytest_main` plus `py_test` directly for a custom main.

    Args:
      kwargs: the extra attributes the caller forwarded to `python_test`.

    Returns:
      The rejection diagnostic string, or `None` when the kwargs are clean.
    """
    if "main" in kwargs:
        return ("python_test always runs pytest and provides its own " +
                "entrypoint; `main` is not supported (generic mains and " +
                "alternate test drivers are rejected per " +
                "docs/testing/generation.md). Use py_pytest_main + py_test " +
                "directly for a custom main.")
    return None

def python_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `py_pytest_test` (M14).

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. Imported non-test modules retain their ordinary
    library owners. Uses pytest and Bazel's standard test and coverage
    protocols per the Python generation contract.

    Args:
      name: public test target name (upstream target is name_upstream).
      srcs: direct test sources collected by pytest.
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream py_pytest_test
        (deps must include the pytest package, e.g. `@pypi//pytest`).
    """
    test_srcs = srcs if srcs != None else []
    rejection = python_test_rejection(kwargs)
    if rejection != None:
        fail(rejection)
    upstream_kwargs = dict(kwargs)
    upstream_kwargs.setdefault("tags", ["manual"])

    # The private test is an implementation detail: tag it manual so
    # `bazel test //...` exercises the public wrapper target only. Preserve
    # caller tags by appending.
    if "tags" in kwargs:
        upstream_kwargs["tags"] = list(kwargs["tags"]) + ["manual"]
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
