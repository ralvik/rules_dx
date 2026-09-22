"""Experimental minimal Scala wrappers (ADR 0019).

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
Upstream: rules_scala 7.3.0 plus Scala 2.13.18 plus ScalaTest 3.2.20 (MODULE.bazel).
"""

load("@rules_java//java:defs.bzl", "JavaInfo")
load("@rules_scala//scala:scala.bzl", _scala_binary = "scala_binary", _scala_library = "scala_library", _scala_test = "scala_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap", "dx_wrap_binary", "dx_wrap_test")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_SCALA_LIBRARY_PROVIDES = [
    JavaInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream `JavaInfo`,
# `InstrumentedFilesInfo`, `OutputGroupInfo`, and `RunEnvironmentInfo`
# at runtime when present, but advertise only `DefaultInfo` plus
# `QualitySourcesInfo`: neither shape is depended on as a Scala library,
# so no consumer matches on the forwarded providers. `QualitySourcesInfo`
# is advertised so quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `java_*` test forwarder).
_DX_SCALA_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

# Scala owns `.scala` only; same-unit `.java` stays `java` per the
# admissibility table (See: docs/quality/quality-sources.md). Java sources
# belong in a `java_library`, so mixed targets keep one owner per file.
_DX_SCALA_SOURCE_SPECS = [("scala", "scala")]
_DX_SCALA_SOURCE_EXTS = [".scala"]

_scala_library_forward = dx_library_forward_rule(
    provides = _DX_SCALA_LIBRARY_PROVIDES,
    required_providers = [(JavaInfo, "JavaInfo")],
    quality_specs = _DX_SCALA_SOURCE_SPECS,
    what = "scala_*",
    allow_files = _DX_SCALA_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Forwards upstream Scala library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Scala sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream scala_library target whose providers are preserved.",
)

_scala_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_SCALA_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_SCALA_SOURCE_SPECS,
    what = "scala_*",
    allow_files = _DX_SCALA_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Executable forwarder for scala_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Scala sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream scala_binary target whose executable is symlinked.",
    optional_providers = [JavaInfo],
    runtime = "besteffort",
)

_scala_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_SCALA_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_SCALA_SOURCE_SPECS,
    what = "scala_*",
    allow_files = _DX_SCALA_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Test forwarder for scala_test: symlinks the upstream test executable.",
    srcs_doc = "Direct Scala test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream scala_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [JavaInfo],
)

def scala_scalacopts_with_werror(kwargs):
    """Returns kwargs with -Xfatal-warnings enforced on scalacopts.

    Existing flags are kept; a missing flag is appended.
    See: docs/testing/generation.md."""
    upstream_kwargs = dict(kwargs)
    scalacopts = list(upstream_kwargs.get("scalacopts", []))
    if "-Xfatal-warnings" not in scalacopts:
        scalacopts = scalacopts + ["-Xfatal-warnings"]
    upstream_kwargs["scalacopts"] = scalacopts
    return upstream_kwargs

def _scala_with_werror(kwargs):
    return scala_scalacopts_with_werror(kwargs)

def _scala_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _scala_library, _scala_library_forward, srcs, visibility = visibility, **_scala_with_werror(kwargs))

def _scala_wrap_binary(name, srcs, visibility = None, **kwargs):
    dx_wrap_binary(name, _scala_binary, _scala_binary_forward, srcs, visibility = visibility, upstream_kwargs = _scala_with_werror(kwargs), **kwargs)

def scala_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `scala_library`."""
    _scala_wrap_library(name, srcs, visibility = visibility, **kwargs)

def scala_binary(name, srcs = None, main_class = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `scala_binary`.

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library and
    names its `main_class` explicitly (no inference); a thin entry binary
    carries only `runtime_deps` with no `srcs` and reports no direct
    sources. Both shapes preserve the upstream providers and execution
    semantics."""
    effective_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)
    if main_class != None:
        upstream_kwargs["main_class"] = main_class
    _scala_wrap_binary(name, effective_srcs, visibility = visibility, **upstream_kwargs)

def scala_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `scala_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. Uses Bazel's
    standard test and coverage protocols over the bundled ScalaTest
 toolchain (ScalaTest 3.2.20 via the managed Coursier route,;
    no Maven lock members needed for the hello closure)."""
    dx_wrap_test(name, _scala_test, _scala_forward_test, srcs, visibility = visibility, upstream_kwargs = _scala_with_werror(kwargs), **kwargs)
