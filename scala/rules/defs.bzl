"""Experimental minimal Scala wrappers (M23, O31, ADR 0019).

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
"""

load("@rules_java//java:defs.bzl", "JavaInfo")
load("@rules_scala//scala:scala.bzl", _scala_binary = "scala_binary", _scala_library = "scala_library", _scala_test = "scala_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap")
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
# is advertised so M23+ quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `java_*` test forwarder).
_DX_SCALA_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_DX_SCALA_SOURCE_SPECS = [("scala", "scala"), ("java", "java")]
_DX_SCALA_SOURCE_EXTS = [".scala", ".java"]

_scala_library_forward = dx_library_forward_rule(
    provides = _DX_SCALA_LIBRARY_PROVIDES,
    required_providers = [(JavaInfo, "JavaInfo")],
    quality_specs = _DX_SCALA_SOURCE_SPECS,
    what = "scala_*",
    allow_files = _DX_SCALA_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Forwards upstream Scala library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Scala/Java sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream scala_library target whose providers are preserved.",
)

_scala_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_SCALA_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_SCALA_SOURCE_SPECS,
    what = "scala_*",
    allow_files = _DX_SCALA_SOURCE_EXTS,
    upstream_providers = None,
    doc = "Executable forwarder for scala_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Scala/Java sources owned by this wrapper for QualitySourcesInfo.",
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
    upstream_providers = None,
    doc = "Test forwarder for scala_test: symlinks the upstream test executable.",
    srcs_doc = "Direct Scala/Java test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream scala_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [JavaInfo],
)

def _scala_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _scala_library, _scala_library_forward, srcs, visibility = visibility, **kwargs)

def _scala_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = dict(kwargs)
    if len(srcs) > 0:
        upstream_kwargs["srcs"] = srcs
    _scala_binary(
        name = name + "_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    _scala_binary_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
        **({"aspect_hints": kwargs["aspect_hints"]} if "aspect_hints" in kwargs else {})
    )

def scala_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `scala_library` (M23)."""
    _scala_wrap_library(name, srcs, visibility = visibility, **kwargs)

def scala_binary(name, srcs = None, main_class = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `scala_binary` (M23).

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
    """Experimental minimal wrapper over `scala_test` (M23).

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. Uses Bazel's
    standard test and coverage protocols over the bundled ScalaTest
    toolchain (no Maven lock members needed for the hello closure)."""
    test_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)

    # The private upstream test stays an implementation detail via private
    # visibility; both it and the public wrapper run under `bazel test //...`
    # (issue #406: no manual; double-execution is the cost of green suites).
    if "tags" in kwargs:
        upstream_kwargs["tags"] = list(kwargs["tags"])
    elif "tags" in upstream_kwargs:
        upstream_kwargs.pop("tags")
    upstream_kwargs["visibility"] = ["//visibility:private"]
    if srcs != None:
        upstream_kwargs["srcs"] = srcs
    _scala_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    _scala_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
        **({"aspect_hints": kwargs["aspect_hints"]} if "aspect_hints" in kwargs else {})
    )
