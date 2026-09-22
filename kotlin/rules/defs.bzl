"""Experimental minimal Kotlin wrappers (ADR 0019).

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
"""

load("@rules_java//java:defs.bzl", "JavaInfo")
load("@rules_kotlin//kotlin:jvm.bzl", _kt_jvm_binary = "kt_jvm_binary", _kt_jvm_library = "kt_jvm_library", _kt_jvm_test = "kt_jvm_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_forwarded_test_kwargs", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_KOTLIN_LIBRARY_PROVIDES = [
    JavaInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream `JavaInfo`,
# `InstrumentedFilesInfo`, `OutputGroupInfo`, and `RunEnvironmentInfo`
# at runtime when present, but advertise only `DefaultInfo` plus
# `QualitySourcesInfo`: neither shape is depended on as a Kotlin library,
# so no consumer matches on the forwarded providers. `QualitySourcesInfo`
# is advertised so quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `java_*` test forwarder).
_DX_KOTLIN_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

# Kotlin owns `.kt`/`.kts` only; same-unit `.java` stays `java` per the
# admissibility table (See: docs/quality/quality-sources.md). Java sources
# belong in a `java_library`, so mixed targets keep one owner per file.
_DX_KOTLIN_SOURCE_SPECS = [("kotlin", ["kt", "kts"])]
_DX_KOTLIN_SOURCE_EXTS = [".kt", ".kts"]

_kotlin_library_forward = dx_library_forward_rule(
    provides = _DX_KOTLIN_LIBRARY_PROVIDES,
    required_providers = [(JavaInfo, "JavaInfo")],
    quality_specs = _DX_KOTLIN_SOURCE_SPECS,
    what = "kotlin_*",
    allow_files = _DX_KOTLIN_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Forwards upstream Kotlin library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Kotlin sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream kt_jvm_library target whose providers are preserved.",
)

_kotlin_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_KOTLIN_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_KOTLIN_SOURCE_SPECS,
    what = "kotlin_*",
    allow_files = _DX_KOTLIN_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Executable forwarder for kotlin_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Kotlin sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream kt_jvm_binary target whose executable is symlinked.",
    optional_providers = [JavaInfo],
    runtime = "besteffort",
)

_kotlin_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_KOTLIN_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_KOTLIN_SOURCE_SPECS,
    what = "kotlin_*",
    allow_files = _DX_KOTLIN_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Test forwarder for kotlin_test: symlinks the upstream test executable.",
    srcs_doc = "Direct Kotlin test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream kt_jvm_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [JavaInfo],
)

def kotlin_kotlinc_opts_with_werror(kwargs):
    """Returns kwargs defaulting kotlinc_opts to warnings_as_errors.

    Caller-provided opts win; only a missing key gets the default.
    See: docs/testing/generation.md."""
    upstream_kwargs = dict(kwargs)
    upstream_kwargs.setdefault("kotlinc_opts", "//kotlin/rules:warnings_as_errors")
    return upstream_kwargs

def _kotlin_with_werror(kwargs):
    return kotlin_kotlinc_opts_with_werror(kwargs)

def _kotlin_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _kt_jvm_library, _kotlin_library_forward, srcs, visibility = visibility, **_kotlin_with_werror(kwargs))

def _kotlin_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = _kotlin_with_werror(kwargs)
    if len(srcs) > 0:
        upstream_kwargs["srcs"] = srcs
    _kt_jvm_binary(
        name = name + "_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    forward_kwargs = {}
    if "tags" in kwargs:
        forward_kwargs["tags"] = kwargs["tags"]
    if "aspect_hints" in kwargs:
        forward_kwargs["aspect_hints"] = kwargs["aspect_hints"]
    _kotlin_binary_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
        **forward_kwargs
    )

def kotlin_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `kt_jvm_library`."""
    _kotlin_wrap_library(name, srcs, visibility = visibility, **kwargs)

def kotlin_binary(name, srcs = None, main_class = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `kt_jvm_binary`.

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library and
    names its `main_class` explicitly (no inference); a thin entry binary
    carries only `runtime_deps` with no `srcs` and reports no direct
    sources. Both shapes preserve the upstream providers and execution
    semantics."""
    effective_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)
    if main_class != None:
        upstream_kwargs["main_class"] = main_class
    _kotlin_wrap_binary(name, effective_srcs, visibility = visibility, **upstream_kwargs)

def kotlin_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `kt_jvm_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. Uses Bazel's
    standard test and coverage protocols."""
    test_srcs = srcs if srcs != None else []
    upstream_kwargs = _kotlin_with_werror(kwargs)

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
    _kt_jvm_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    forward_kwargs = dx_forwarded_test_kwargs(kwargs)
    if "aspect_hints" in kwargs:
        forward_kwargs["aspect_hints"] = kwargs["aspect_hints"]
    _kotlin_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
        **forward_kwargs
    )
