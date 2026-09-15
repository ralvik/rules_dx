"""Experimental minimal F# wrappers (M23, O31, ADR 0019).

Thin conventional boundary over the pinned `rules_dotnet 0.22.1` ruleset
(one upstream covers both admitted .NET languages; C# wrappers live under
//csharp). Each `fsharp_*` macro creates one private
`<name>_upstream` target with the passed attributes and one public
`<name>` forwarding target. The library forwarder preserves the upstream
providers (`DotnetAssemblyCompileInfo`, `DotnetAssemblyRuntimeInfo`,
`DefaultInfo`) unchanged and adds `QualitySourcesInfo` normalized from the
wrapper's direct `srcs`. Binaries and tests use an executable forwarder
whose own symlink action points at the upstream executable (Bazel requires
executable-providing rules to create the file themselves).

Used upstream symbols (`@rules_dotnet//dotnet:defs.bzl`): `fsharp_library`,
`fsharp_binary`, `fsharp_test`. The used providers
(`DotnetAssemblyCompileInfo`, `DotnetAssemblyRuntimeInfo`) load from the
upstream private providers module (`@rules_dotnet//dotnet/private:providers.bzl`):
no public provider module exists, so this narrow load is the boundary (same
shape as the `java_*` load of the upstream java-info module). No other
upstream surface is used; consumers needing more (`fsharp_nunit_test`,
`publish_binary`, `import_library`, `dotnet_tool`) load the upstream module
directly.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"fsharp": <direct .fs/.fsi>})`.
Reference/runtime assemblies and toolchain closures stay readable from the
preserved `DotnetAssembly*` providers; no second provider duplicates them.

`target_frameworks` defaults to `["net10.0"]` (the pinned SDK 10.0.201
toolchain in MODULE.bazel) and passes through when set explicitly; no other
toolchain/TFM selection lives here. Tests are plain `fsharp_test`
executables (exit code is the verdict); the xUnit/NUnit runner selection
and Paket/NuGet lock wiring stay open under O31 per the provisional
support-matrix tables. Wrappers accept no SDK version fields; unknown
frameworks fail in upstream toolchain resolution, never here. Only `.fs`
and `.fsi` sources are accepted. Per-platform SDK acquisition selection stays open
under O31.
"""

load("@rules_dotnet//dotnet:defs.bzl", _fsharp_binary = "fsharp_binary", _fsharp_library = "fsharp_library", _fsharp_test = "fsharp_test")
load("@rules_dotnet//dotnet/private:providers.bzl", "DotnetAssemblyCompileInfo", "DotnetAssemblyRuntimeInfo")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_FSHARP_LIBRARY_PROVIDES = [
    DotnetAssemblyCompileInfo,
    DotnetAssemblyRuntimeInfo,
    DefaultInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream assembly infos,
# `InstrumentedFilesInfo`, `OutputGroupInfo`, and `RunEnvironmentInfo`
# at runtime when present, but advertise only `DefaultInfo` plus
# `QualitySourcesInfo`: neither shape is depended on as a F# library,
# so no consumer matches on the forwarded providers. `QualitySourcesInfo`
# is advertised so M23+ quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `java_*` test forwarder).
_DX_FSHARP_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_DX_FSHARP_SOURCE_SPECS = [("fsharp", ["fs", "fsi"])]
_DX_FSHARP_SOURCE_EXTS = [".fs", ".fsi"]

_fsharp_library_forward = dx_library_forward_rule(
    provides = _DX_FSHARP_LIBRARY_PROVIDES,
    required_providers = [(DotnetAssemblyCompileInfo, "DotnetAssemblyCompileInfo"), (DotnetAssemblyRuntimeInfo, "DotnetAssemblyRuntimeInfo")],
    quality_specs = _DX_FSHARP_SOURCE_SPECS,
    what = "fsharp_*",
    allow_files = _DX_FSHARP_SOURCE_EXTS,
    upstream_providers = [[DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo]],
    doc = "Forwards upstream F# library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct F# sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream fsharp_library target whose providers are preserved.",
    runtime = "besteffort",
)

_fsharp_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_FSHARP_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_FSHARP_SOURCE_SPECS,
    what = "fsharp_*",
    allow_files = _DX_FSHARP_SOURCE_EXTS,
    upstream_providers = None,
    doc = "Executable forwarder for fsharp_binary: symlinks the upstream binary.",
    srcs_doc = "Direct F# sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream fsharp_binary target whose executable is symlinked.",
    optional_providers = [DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo],
    runtime = "besteffort",
)

_fsharp_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_FSHARP_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_FSHARP_SOURCE_SPECS,
    what = "fsharp_*",
    allow_files = _DX_FSHARP_SOURCE_EXTS,
    upstream_providers = None,
    doc = "Test forwarder for fsharp_test: symlinks the upstream test executable.",
    srcs_doc = "Direct F# test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream fsharp_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo],
    runtime = "besteffort",
)

def _fsharp_with_tfm(kwargs):
    upstream_kwargs = dict(kwargs)
    upstream_kwargs.setdefault("target_frameworks", ["net10.0"])
    return upstream_kwargs

def _fsharp_wrap_library(name, srcs, visibility = None, **kwargs):
    _fsharp_library(
        name = name + "_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **_fsharp_with_tfm(kwargs)
    )
    _fsharp_library_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def _fsharp_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = _fsharp_with_tfm(kwargs)
    if len(srcs) > 0:
        upstream_kwargs["srcs"] = srcs
    _fsharp_binary(
        name = name + "_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    _fsharp_binary_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def fsharp_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `fsharp_library` (M23).

    Args:
      name: public library target name (upstream target is name_upstream).
      srcs: direct F# sources owned by this wrapper.
      visibility: visibility of the public forwarding library target.
      **kwargs: extra attributes forwarded to the upstream fsharp_library
        (deps, data; target_frameworks defaults to ["net10.0"]).
    """
    _fsharp_wrap_library(name, srcs, visibility = visibility, **kwargs)

def fsharp_binary(name, srcs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `fsharp_binary` (M23).

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library;
    the entry point follows the F# `[<EntryPoint>] let main` convention (no
    main_class attribute exists upstream). Both shapes preserve the upstream providers
    and execution semantics.

    Args:
      name: public binary target name (upstream target is name_upstream).
      srcs: direct binary sources.
      visibility: visibility of the public forwarding binary target.
      **kwargs: extra attributes forwarded to the upstream fsharp_binary.
    """
    effective_srcs = srcs if srcs != None else []
    _fsharp_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def fsharp_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `fsharp_test` (M23).

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. The test is a
    plain executable (exit code is the verdict); the xUnit/NUnit runner
    selection stays open under O31. Uses Bazel's standard test and coverage
    protocols.

    Args:
      name: public test target name (upstream target is name_upstream).
      srcs: direct test sources.
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream fsharp_test
        (deps must name the wrapper library under test).
    """
    test_srcs = srcs if srcs != None else []
    upstream_kwargs = _fsharp_with_tfm(kwargs)
    upstream_kwargs.setdefault("tags", ["manual"])

    # The private test is an implementation detail: tag it manual so
    # `bazel test //...` exercises the public wrapper target only. Preserve
    # caller tags by appending.
    if "tags" in kwargs:
        upstream_kwargs["tags"] = list(kwargs["tags"]) + ["manual"]
    upstream_kwargs["visibility"] = ["//visibility:private"]
    if srcs != None:
        upstream_kwargs["srcs"] = srcs
    _fsharp_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    _fsharp_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
    )
