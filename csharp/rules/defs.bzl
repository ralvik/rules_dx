"""Experimental minimal C# wrappers (M23, O31, ADR 0019).

Thin conventional boundary over the pinned `rules_dotnet 0.22.1` ruleset
(one upstream covers both admitted .NET languages; F# wrappers live under
//fsharp). Each `dx_csharp_*` macro creates one private
`<name>_dx_upstream` target with the passed attributes and one public
`<name>` forwarding target. The library forwarder preserves the upstream
providers (`DotnetAssemblyCompileInfo`, `DotnetAssemblyRuntimeInfo`,
`DefaultInfo`) unchanged and adds `QualitySourcesInfo` normalized from the
wrapper's direct `srcs`. Binaries and tests use an executable forwarder
whose own symlink action points at the upstream executable (Bazel requires
executable-providing rules to create the file themselves).

Used upstream symbols (`@rules_dotnet//dotnet:defs.bzl`): `csharp_library`,
`csharp_binary`, `csharp_test`. The used providers
(`DotnetAssemblyCompileInfo`, `DotnetAssemblyRuntimeInfo`) load from the
upstream private providers module (`@rules_dotnet//dotnet/private:providers.bzl`):
no public provider module exists, so this narrow load is the boundary (same
shape as the `dx_java_*` load of the upstream java-info module). No other
upstream surface is used; consumers needing more (`csharp_nunit_test`,
`publish_binary`, `import_library`, `dotnet_tool`) load the upstream module
directly.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"csharp": <direct .cs>})`.
Reference/runtime assemblies and toolchain closures stay readable from the
preserved `DotnetAssembly*` providers; no second provider duplicates them.

`target_frameworks` defaults to `["net10.0"]` (the pinned SDK 10.0.201
toolchain in MODULE.bazel) and passes through when set explicitly; no other
toolchain/TFM selection lives here. Tests are plain `csharp_test`
executables (exit code is the verdict); the xUnit/NUnit runner selection
and Paket/NuGet lock wiring stay open under O31 per the provisional
support-matrix tables. Wrappers accept no SDK version fields; unknown
frameworks fail in upstream toolchain resolution, never here. Only `.cs`
sources are accepted. Per-platform SDK acquisition selection stays open
under O31.
"""

load("@rules_dotnet//dotnet:defs.bzl", _csharp_binary = "csharp_binary", _csharp_library = "csharp_library", _csharp_test = "csharp_test")
load("@rules_dotnet//dotnet/private:providers.bzl", "DotnetAssemblyCompileInfo", "DotnetAssemblyRuntimeInfo")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

_DX_CSHARP_LIBRARY_PROVIDES = [
    DotnetAssemblyCompileInfo,
    DotnetAssemblyRuntimeInfo,
    DefaultInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream assembly infos,
# `InstrumentedFilesInfo`, `OutputGroupInfo`, and `RunEnvironmentInfo`
# at runtime when present, but advertise only `DefaultInfo` plus
# `QualitySourcesInfo`: neither shape is depended on as a C# library,
# so no consumer matches on the forwarded providers. `QualitySourcesInfo`
# is advertised so M23+ quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `dx_java_*` test forwarder).
_DX_CSHARP_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

def _dx_csharp_quality_sources(ctx):
    csharp = [f for f in ctx.files.srcs if f.extension == "cs"]
    direct_sources = {}
    if len(csharp) > 0:
        direct_sources["csharp"] = depset(csharp)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _dx_csharp_preserved_library_providers(ctx):
    upstream = ctx.attr.upstream
    if DotnetAssemblyCompileInfo not in upstream:
        fail("dx_csharp_*: upstream target has no DotnetAssemblyCompileInfo: " + str(ctx.attr.upstream.label))
    if DotnetAssemblyRuntimeInfo not in upstream:
        fail("dx_csharp_*: upstream target has no DotnetAssemblyRuntimeInfo: " + str(ctx.attr.upstream.label))
    return [upstream[DotnetAssemblyCompileInfo], upstream[DotnetAssemblyRuntimeInfo]]

def _dx_csharp_forwarded_output_providers(ctx):
    """Output groups and run env forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    out = []
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _dx_csharp_forwarded_assembly_infos(ctx):
    """Upstream assembly infos forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    out = []
    if DotnetAssemblyCompileInfo in upstream:
        out.append(upstream[DotnetAssemblyCompileInfo])
    if DotnetAssemblyRuntimeInfo in upstream:
        out.append(upstream[DotnetAssemblyRuntimeInfo])
    return out

def _dx_csharp_forwarded_instrumented(ctx):
    """Upstream `InstrumentedFilesInfo` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo in upstream:
        return [upstream[InstrumentedFilesInfo]]
    return []

def _dx_csharp_library_forward_impl(ctx):
    upstream = ctx.attr.upstream
    return (
        _dx_csharp_preserved_library_providers(ctx) +
        [upstream[DefaultInfo]] +
        _dx_csharp_forwarded_instrumented(ctx) +
        _dx_csharp_forwarded_output_providers(ctx) +
        [_dx_csharp_quality_sources(ctx)]
    )

_dx_csharp_library_forward = rule(
    implementation = _dx_csharp_library_forward_impl,
    provides = _DX_CSHARP_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".cs"],
            doc = "Direct C# sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[DotnetAssemblyCompileInfo, DotnetAssemblyRuntimeInfo]],
            doc = "The private upstream csharp_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream C# library providers unchanged and adds QualitySourcesInfo.",
)

def _dx_csharp_symlink_default_info(ctx):
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail("dx_csharp_*: upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def _dx_csharp_binary_forward_impl(ctx):
    return (
        [_dx_csharp_symlink_default_info(ctx)] +
        _dx_csharp_forwarded_assembly_infos(ctx) +
        _dx_csharp_forwarded_instrumented(ctx) +
        _dx_csharp_forwarded_output_providers(ctx) +
        [_dx_csharp_quality_sources(ctx)]
    )

_dx_csharp_binary_forward = rule(
    implementation = _dx_csharp_binary_forward_impl,
    executable = True,
    provides = _DX_CSHARP_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".cs"],
            doc = "Direct C# sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            doc = "The private upstream csharp_binary target whose executable is symlinked.",
        ),
    },
    doc = "Executable forwarder for dx_csharp_binary: symlinks the upstream binary.",
)

def _dx_csharp_test_forward_impl(ctx):
    return (
        [_dx_csharp_symlink_default_info(ctx)] +
        _dx_csharp_forwarded_assembly_infos(ctx) +
        _dx_csharp_forwarded_instrumented(ctx) +
        _dx_csharp_forwarded_output_providers(ctx) +
        [_dx_csharp_quality_sources(ctx)]
    )

_dx_csharp_forward_test = rule(
    implementation = _dx_csharp_test_forward_impl,
    test = True,
    provides = _DX_CSHARP_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".cs"],
            doc = "Direct C# test sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            doc = "The private upstream csharp_test target whose executable is symlinked.",
        ),
        "_lcov_merger": attr.label(
            default = configuration_field(fragment = "coverage", name = "output_generator"),
            executable = True,
            cfg = "exec",
            doc = "Coverage-report merger. Bazel's coverage runner passes " +
                  "this magic attribute as LCOV_MERGER, which merges the " +
                  "per-test staging report into coverage.dat; without it " +
                  "the runner exits after touching an empty file even " +
                  "though the test collected coverage. Same declaration as " +
                  "the upstream-wrapping test forwarders.",
        ),
    },
    doc = "Test forwarder for dx_csharp_test: symlinks the upstream test executable.",
)

def _dx_csharp_with_tfm(kwargs):
    upstream_kwargs = dict(kwargs)
    upstream_kwargs.setdefault("target_frameworks", ["net10.0"])
    return upstream_kwargs

def _dx_csharp_wrap_library(name, srcs, visibility = None, **kwargs):
    _csharp_library(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **_dx_csharp_with_tfm(kwargs)
    )
    _dx_csharp_library_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def _dx_csharp_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = _dx_csharp_with_tfm(kwargs)
    if len(srcs) > 0:
        upstream_kwargs["srcs"] = srcs
    _csharp_binary(
        name = name + "_dx_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    _dx_csharp_binary_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def dx_csharp_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `csharp_library` (M23).

    Args:
      name: public library target name (upstream target is name_dx_upstream).
      srcs: direct C# sources owned by this wrapper.
      visibility: visibility of the public forwarding library target.
      **kwargs: extra attributes forwarded to the upstream csharp_library
        (deps, data; target_frameworks defaults to ["net10.0"]).
    """
    _dx_csharp_wrap_library(name, srcs, visibility = visibility, **kwargs)

def dx_csharp_binary(name, srcs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `csharp_binary` (M23).

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library;
    the entry point follows the C# `Main` convention (no main_class
    attribute exists upstream). Both shapes preserve the upstream providers
    and execution semantics.

    Args:
      name: public binary target name (upstream target is name_dx_upstream).
      srcs: direct binary sources.
      visibility: visibility of the public forwarding binary target.
      **kwargs: extra attributes forwarded to the upstream csharp_binary.
    """
    effective_srcs = srcs if srcs != None else []
    _dx_csharp_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def dx_csharp_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `csharp_test` (M23).

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. The test is a
    plain executable (exit code is the verdict); the xUnit/NUnit runner
    selection stays open under O31. Uses Bazel's standard test and coverage
    protocols.

    Args:
      name: public test target name (upstream target is name_dx_upstream).
      srcs: direct test sources.
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream csharp_test
        (deps must name the wrapper library under test).
    """
    test_srcs = srcs if srcs != None else []
    upstream_kwargs = _dx_csharp_with_tfm(kwargs)
    upstream_kwargs.setdefault("tags", ["manual"])

    # The private test is an implementation detail: tag it manual so
    # `bazel test //...` exercises the public wrapper target only. Preserve
    # caller tags by appending.
    if "tags" in kwargs:
        upstream_kwargs["tags"] = list(kwargs["tags"]) + ["manual"]
    upstream_kwargs["visibility"] = ["//visibility:private"]
    if srcs != None:
        upstream_kwargs["srcs"] = srcs
    _csharp_test(
        name = name + "_dx_upstream",
        **upstream_kwargs
    )
    _dx_csharp_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_dx_upstream",
        srcs = test_srcs,
        visibility = visibility,
    )
