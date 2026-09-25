"""Experimental minimal C# wrappers (ADR 0019).

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
"""

load("@rules_dotnet//dotnet:defs.bzl", _csharp_binary = "csharp_binary", _csharp_library = "csharp_library", _csharp_test = "csharp_test")

# Intentional upstream-private load (issue #928): the .NET assembly
# providers live only under @rules_dotnet//dotnet/private, so the wrapper
# must load them there; the sealed `upstream_providers` plus
# `required_providers` below keep the boundary fail-closed.
load("@rules_dotnet//dotnet/private:providers.bzl", "DotnetAssemblyCompileInfo", "DotnetAssemblyRuntimeInfo")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap", "dx_wrap_binary", "dx_wrap_test")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_CSHARP_LIBRARY_PROVIDES = [
    DotnetAssemblyCompileInfo,
    DotnetAssemblyRuntimeInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream assembly infos,
# `InstrumentedFilesInfo`, `OutputGroupInfo`, and `RunEnvironmentInfo`
# at runtime when present, but advertise only `DefaultInfo` plus
# `QualitySourcesInfo`: neither shape is depended on as a C# library,
# so no consumer matches on the forwarded providers. `QualitySourcesInfo`
# is advertised so quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `java_*` test forwarder).
_DX_CSHARP_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_DX_CSHARP_SOURCE_SPECS = [("csharp", "cs")]
_DX_CSHARP_SOURCE_EXTS = [".cs"]

_csharp_library_forward = dx_library_forward_rule(
    provides = _DX_CSHARP_LIBRARY_PROVIDES,
    required_providers = [(DotnetAssemblyCompileInfo, "DotnetAssemblyCompileInfo"), (DotnetAssemblyRuntimeInfo, "DotnetAssemblyRuntimeInfo")],
    quality_specs = _DX_CSHARP_SOURCE_SPECS,
    what = "csharp_*",
    allow_files = _DX_CSHARP_SOURCE_EXTS,
    upstream_providers = [[DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo]],
    doc = "Forwards upstream C# library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct C# sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream csharp_library target whose providers are preserved.",
    runtime = "besteffort",
)

_csharp_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_CSHARP_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_CSHARP_SOURCE_SPECS,
    what = "csharp_*",
    allow_files = _DX_CSHARP_SOURCE_EXTS,
    upstream_providers = [[DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo]],
    doc = "Executable forwarder for csharp_binary: symlinks the upstream binary.",
    srcs_doc = "Direct C# sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream csharp_binary target whose executable is symlinked.",
    optional_providers = [DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo],
    runtime = "besteffort",
)

_csharp_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_CSHARP_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_CSHARP_SOURCE_SPECS,
    what = "csharp_*",
    allow_files = _DX_CSHARP_SOURCE_EXTS,
    upstream_providers = [[DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo]],
    doc = "Test forwarder for csharp_test: symlinks the upstream test executable.",
    srcs_doc = "Direct C# test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream csharp_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo],
    runtime = "besteffort",
)

def csharp_tfm_with_defaults(kwargs):
    """Returns kwargs defaulting target_frameworks plus warnings-as-errors.

    Caller-provided values win; only missing keys get defaults.
    The default is single-TFM (`net10.0`); multi-pivot consumers declare
    one `csharp_library` per TFM and aggregate per-pivot SARIFs in
    deterministic pivot order.
    See: docs/testing/generation.md, csharp/tests/fixtures/roslyn/pins.bzl.
    """
    upstream_kwargs = dict(kwargs)
    upstream_kwargs.setdefault("target_frameworks", ["net10.0"])
    upstream_kwargs.setdefault("treat_warnings_as_errors", True)
    return upstream_kwargs

def _csharp_with_tfm(kwargs):
    return csharp_tfm_with_defaults(kwargs)

def _csharp_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _csharp_library, _csharp_library_forward, srcs, visibility = visibility, **_csharp_with_tfm(kwargs))

def _csharp_wrap_binary(name, srcs, visibility = None, **kwargs):
    dx_wrap_binary(name, _csharp_binary, _csharp_binary_forward, srcs, visibility = visibility, upstream_kwargs = _csharp_with_tfm(kwargs), **kwargs)

def csharp_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `csharp_library`.

    Defaults to single-TFM `net10.0`; pass explicit `target_frameworks`
    for any other pivot (one library per TFM, see the roslyn fixture)."""
    _csharp_wrap_library(name, srcs, visibility = visibility, **kwargs)

def csharp_binary(name, srcs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `csharp_binary`.

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library;
    the entry point follows the C# `Main` convention (no main_class
    attribute exists upstream). Both shapes preserve the upstream providers
    and execution semantics."""
    effective_srcs = srcs if srcs != None else []
    _csharp_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def csharp_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `csharp_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. Plain-executable
    tests (exit code is the verdict) stay supported; the xUnit v3 4.0.0
 mapping is qualified (`csharp/tests/fixtures/xunit/`:
    `[Fact]`/`[Theory]` sources plus checked-in MTP entry-point shims over
    the pinned `@paket.main//xunit.v3` closure; unpinned runner rejected).
    Uses Bazel's standard test and coverage protocols."""
    dx_wrap_test(name, _csharp_test, _csharp_forward_test, srcs, visibility = visibility, upstream_kwargs = _csharp_with_tfm(kwargs), **kwargs)
