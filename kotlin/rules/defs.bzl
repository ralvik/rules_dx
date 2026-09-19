"""Experimental minimal Kotlin wrappers (M23, O31, ADR 0019).

Thin conventional boundary over the pinned `rules_kotlin 2.4.10` ruleset.
Each `kotlin_*` macro creates one private `<name>_upstream` target with
the passed attributes and one public `<name>` forwarding target. The library
forwarder preserves the upstream providers (`JavaInfo`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `srcs`. Binaries and tests use an
executable forwarder whose own symlink action points at the upstream
executable (Bazel requires executable-providing rules to create the
file themselves).

Used upstream symbols (`@rules_kotlin//kotlin:jvm.bzl`): `kt_jvm_library`,
`kt_jvm_binary`, `kt_jvm_test`. The preserved provider (`JavaInfo`) is
Bazel's native Java provider, which `kt_jvm_*` rules also provide alongside
their Kotlin-internal `KtJvmInfo` (kept readable from the private upstream
target, never duplicated here). No other upstream surface is used;
consumers needing more (`kt_jvm_import`, compiler plugins) load the
upstream module directly.

Normalization is deliberately narrow: direct `.kt` sources map to the
`kotlin` class and direct `.java` sources (same-compilation-unit mixed
sources, which upstream compiles Java-first) map to the `java` class:
`QualitySourcesInfo(direct_sources = {"kotlin": <direct .kt>,
"java": <direct .java>})`. Empty classes are omitted. Classpaths,
transitive jars, and toolchain closures stay readable from the preserved
`JavaInfo`; no second provider duplicates them.

`main_class` (binaries) and `test_class` (tests) always pass through with
no invented default. Wrappers accept no toolchain version fields; unknown
versions fail in upstream toolchain resolution, never here. Per-platform
JDK/kotlinc acquisition selection stays open under O31: the wrappers build
on the default toolchains now, and the hermetic selection from the support
matrix is qualified separately.
"""

load("@rules_java//java:defs.bzl", "JavaInfo")
load("@rules_kotlin//kotlin:jvm.bzl", _kt_jvm_binary = "kt_jvm_binary", _kt_jvm_library = "kt_jvm_library", _kt_jvm_test = "kt_jvm_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap")
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
# is advertised so M23+ quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `java_*` test forwarder).
_DX_KOTLIN_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_DX_KOTLIN_SOURCE_SPECS = [("kotlin", "kt"), ("java", "java")]
_DX_KOTLIN_SOURCE_EXTS = [".kt", ".java"]

_kotlin_library_forward = dx_library_forward_rule(
    provides = _DX_KOTLIN_LIBRARY_PROVIDES,
    required_providers = [(JavaInfo, "JavaInfo")],
    quality_specs = _DX_KOTLIN_SOURCE_SPECS,
    what = "kotlin_*",
    allow_files = _DX_KOTLIN_SOURCE_EXTS,
    upstream_providers = [[JavaInfo]],
    doc = "Forwards upstream Kotlin library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Kotlin/Java sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream kt_jvm_library target whose providers are preserved.",
)

_kotlin_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_KOTLIN_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_KOTLIN_SOURCE_SPECS,
    what = "kotlin_*",
    allow_files = _DX_KOTLIN_SOURCE_EXTS,
    upstream_providers = None,
    doc = "Executable forwarder for kotlin_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Kotlin/Java sources owned by this wrapper for QualitySourcesInfo.",
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
    upstream_providers = None,
    doc = "Test forwarder for kotlin_test: symlinks the upstream test executable.",
    srcs_doc = "Direct Kotlin/Java test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream kt_jvm_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [JavaInfo],
)

def _kotlin_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _kt_jvm_library, _kotlin_library_forward, srcs, visibility = visibility, **kwargs)

def _kotlin_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = dict(kwargs)
    if len(srcs) > 0:
        upstream_kwargs["srcs"] = srcs
    _kt_jvm_binary(
        name = name + "_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    _kotlin_binary_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
        **({"aspect_hints": kwargs["aspect_hints"]} if "aspect_hints" in kwargs else {})
    )

def kotlin_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `kt_jvm_library` (M23).

    Args:
      name: public library target name (upstream target is name_upstream).
      srcs: direct Kotlin/Java sources owned by this wrapper.
      visibility: visibility of the public forwarding library target.
      **kwargs: extra attributes forwarded to the upstream kt_jvm_library
        (deps, resources, data).
    """
    _kotlin_wrap_library(name, srcs, visibility = visibility, **kwargs)

def kotlin_binary(name, srcs = None, main_class = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `kt_jvm_binary` (M23).

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library and
    names its `main_class` explicitly (no inference); a thin entry binary
    carries only `runtime_deps` with no `srcs` and reports no direct
    sources. Both shapes preserve the upstream providers and execution
    semantics.

    Args:
      name: public binary target name (upstream target is name_upstream).
      srcs: direct binary sources; empty for thin entry binaries.
      main_class: binary entry point, passed through with no default.
      visibility: visibility of the public forwarding binary target.
      **kwargs: extra attributes forwarded to the upstream kt_jvm_binary.
    """
    effective_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)
    if main_class != None:
        upstream_kwargs["main_class"] = main_class
    _kotlin_wrap_binary(name, effective_srcs, visibility = visibility, **upstream_kwargs)

def kotlin_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `kt_jvm_test` (M23).

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. Uses Bazel's
    standard test and coverage protocols.

    Args:
      name: public test target name (upstream target is name_upstream).
      srcs: direct test sources.
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream kt_jvm_test
        (deps must name the wrapper library under test; test_class passes
        through with no default).
    """
    test_srcs = srcs if srcs != None else []
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
    _kt_jvm_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    _kotlin_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
        **({"aspect_hints": kwargs["aspect_hints"]} if "aspect_hints" in kwargs else {})
    )
