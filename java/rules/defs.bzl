"""Experimental minimal Java wrappers (ADR 0019).

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
"""

load("@rules_java//java:defs.bzl", _java_binary = "java_binary", _java_library = "java_library", _java_test = "java_test")
load("@rules_java//java/common:java_info.bzl", "JavaInfo")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap", "dx_wrap_binary", "dx_wrap_test")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_JAVA_LIBRARY_PROVIDES = [
    JavaInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream `JavaInfo`,
# `InstrumentedFilesInfo`, `OutputGroupInfo`, and `RunEnvironmentInfo`
# at runtime when present, but advertise only `DefaultInfo` plus
# `QualitySourcesInfo`: neither shape is depended on as a Java library,
# so no consumer matches on the forwarded providers. `QualitySourcesInfo`
# is advertised so quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `go_*` test forwarder).
_DX_JAVA_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_DX_JAVA_SOURCE_SPECS = [("java", "java")]
_DX_JAVA_SOURCE_EXTS = [".java"]

_java_library_forward = dx_library_forward_rule(
    provides = _DX_JAVA_LIBRARY_PROVIDES,
    required_providers = [(JavaInfo, "JavaInfo")],
    quality_specs = _DX_JAVA_SOURCE_SPECS,
    what = "java_*",
    allow_files = _DX_JAVA_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Forwards upstream Java library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Java sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream java_library target whose providers are preserved.",
)

_java_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_JAVA_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_JAVA_SOURCE_SPECS,
    what = "java_*",
    allow_files = _DX_JAVA_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Executable forwarder for java_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Java sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream java_binary target whose executable is symlinked.",
    optional_providers = [JavaInfo],
    runtime = "besteffort",
)

_java_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_JAVA_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_JAVA_SOURCE_SPECS,
    what = "java_*",
    allow_files = _DX_JAVA_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Test forwarder for java_test: symlinks the upstream test executable.",
    srcs_doc = "Direct Java test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream java_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [JavaInfo],
)

def java_javacopts_with_werror(kwargs):
    """Returns kwargs with -Werror plus -Xlint:all enforced on javacopts.

    Existing flags are kept; missing ones are appended.
    See: docs/testing/generation.md.

    Args:
      kwargs: Rule keyword arguments possibly containing `javacopts`.

    Returns:
      Copy of `kwargs` with `-Werror` and `-Xlint:all` appended when absent.
    """
    upstream_kwargs = dict(kwargs)
    javacopts = list(upstream_kwargs.get("javacopts", []))
    for flag in ["-Werror", "-Xlint:all"]:
        if flag not in javacopts:
            javacopts = javacopts + [flag]
    upstream_kwargs["javacopts"] = javacopts
    return upstream_kwargs

def _java_with_werror(kwargs):
    return java_javacopts_with_werror(kwargs)

def _java_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _java_library, _java_library_forward, srcs, visibility = visibility, **_java_with_werror(kwargs))

def _java_wrap_binary(name, srcs, visibility = None, **kwargs):
    dx_wrap_binary(name, _java_binary, _java_binary_forward, srcs, visibility = visibility, upstream_kwargs = _java_with_werror(kwargs), **kwargs)

def java_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `java_library`."""
    _java_wrap_library(name, srcs, visibility = visibility, **kwargs)

def java_binary(name, srcs = None, main_class = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `java_binary`.

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library and
    names its `main_class` explicitly (no inference); a thin entry binary
    carries only `runtime_deps` with no `srcs` and reports no direct
    sources. Both shapes preserve the upstream providers and execution
    semantics.

    Args:
      name: Target name.
      srcs: Direct Java sources; None becomes an empty list.
      main_class: Fully qualified main class, or None to omit.
      visibility: Visibility list for the public forwarder.
      **kwargs: Forwarded keyword arguments to the wrapper rules.
    """
    effective_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)
    if main_class != None:
        upstream_kwargs["main_class"] = main_class
    _java_wrap_binary(name, effective_srcs, visibility = visibility, **upstream_kwargs)

def java_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `java_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. Uses Bazel's
    standard test and coverage protocols."""
    dx_wrap_test(name, _java_test, _java_forward_test, srcs, visibility = visibility, upstream_kwargs = _java_with_werror(kwargs), **kwargs)
