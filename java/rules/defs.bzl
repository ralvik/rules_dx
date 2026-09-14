"""Experimental minimal Java wrappers (M23, O31, ADR 0019).

Thin conventional boundary over the pinned `rules_java 9.7.0` ruleset.
Each `java_*` macro creates one private `<name>_upstream` target with
the passed attributes and one public `<name>` forwarding target. The library
forwarder preserves the upstream providers (`JavaInfo`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `srcs`. Binaries and tests use an
executable forwarder whose own symlink action points at the upstream
executable (Bazel requires executable-providing rules to create the
file themselves).

Used upstream symbols (`@rules_java//java:defs.bzl`): `java_library`,
`java_binary`, `java_test`. The used provider (`JavaInfo`) is Bazel's
native Java provider. No other upstream surface is used; consumers needing
more (`java_import`, `java_plugin`, `java_package_configuration`) load the
upstream module directly.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"java": <direct .java>})`. Classpaths,
transitive jars, and toolchain closures stay readable from the preserved
`JavaInfo`; no second provider duplicates them.

`main_class` (binaries) and `test_class` (tests) always pass through with
no invented default. Wrappers accept no toolchain version fields; unknown
versions fail in upstream toolchain resolution, never here. Only `.java`
sources are accepted. Per-platform JDK acquisition selection stays open
under O31: the wrappers build on the default toolchain now, and the
hermetic `--java_runtime_version` route from the support matrix is
qualified separately.
"""

load("@rules_java//java:defs.bzl", _java_binary = "java_binary", _java_library = "java_library", _java_test = "java_test")
load("@rules_java//java/common:java_info.bzl", "JavaInfo")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

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
# is advertised so M23+ quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `go_*` test forwarder).
_DX_JAVA_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

def _java_quality_sources(ctx):
    java = [f for f in ctx.files.srcs if f.extension == "java"]
    direct_sources = {}
    if len(java) > 0:
        direct_sources["java"] = depset(java)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _java_preserved_library_providers(ctx):
    upstream = ctx.attr.upstream
    if JavaInfo not in upstream:
        fail("java_*: upstream target has no JavaInfo: " + str(ctx.attr.upstream.label))
    return [upstream[JavaInfo]]

def _java_forwarded_output_providers(ctx):
    """Output groups and run env forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    out = []
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _java_forwarded_java_info(ctx):
    """Upstream `JavaInfo` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if JavaInfo in upstream:
        return [upstream[JavaInfo]]
    return []

def _java_forwarded_instrumented(ctx):
    """Upstream `InstrumentedFilesInfo` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo in upstream:
        return [upstream[InstrumentedFilesInfo]]
    return []

def _java_library_forward_impl(ctx):
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo not in upstream:
        fail("java_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    return (
        _java_preserved_library_providers(ctx) +
        [upstream[DefaultInfo]] +
        [upstream[InstrumentedFilesInfo]] +
        _java_forwarded_output_providers(ctx) +
        [_java_quality_sources(ctx)]
    )

_java_library_forward = rule(
    implementation = _java_library_forward_impl,
    provides = _DX_JAVA_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".java"],
            doc = "Direct Java sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[JavaInfo]],
            doc = "The private upstream java_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream Java library providers unchanged and adds QualitySourcesInfo.",
)

def _java_symlink_default_info(ctx):
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail("java_*: upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def _java_binary_forward_impl(ctx):
    return (
        [_java_symlink_default_info(ctx)] +
        _java_forwarded_java_info(ctx) +
        _java_forwarded_instrumented(ctx) +
        _java_forwarded_output_providers(ctx) +
        [_java_quality_sources(ctx)]
    )

_java_binary_forward = rule(
    implementation = _java_binary_forward_impl,
    executable = True,
    provides = _DX_JAVA_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".java"],
            doc = "Direct Java sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            doc = "The private upstream java_binary target whose executable is symlinked.",
        ),
    },
    doc = "Executable forwarder for java_binary: symlinks the upstream binary.",
)

def _java_test_forward_impl(ctx):
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo not in upstream:
        fail("java_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    return (
        [_java_symlink_default_info(ctx)] +
        [upstream[InstrumentedFilesInfo]] +
        _java_forwarded_java_info(ctx) +
        _java_forwarded_output_providers(ctx) +
        [_java_quality_sources(ctx)]
    )

_java_forward_test = rule(
    implementation = _java_test_forward_impl,
    test = True,
    provides = _DX_JAVA_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".java"],
            doc = "Direct Java test sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            doc = "The private upstream java_test target whose executable is symlinked.",
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
    doc = "Test forwarder for java_test: symlinks the upstream test executable.",
)

def _java_wrap_library(name, srcs, visibility = None, **kwargs):
    _java_library(
        name = name + "_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _java_library_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def _java_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = dict(kwargs)
    if len(srcs) > 0:
        upstream_kwargs["srcs"] = srcs
    _java_binary(
        name = name + "_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    _java_binary_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def java_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `java_library` (M23).

    Args:
      name: public library target name (upstream target is name_upstream).
      srcs: direct Java sources owned by this wrapper.
      visibility: visibility of the public forwarding library target.
      **kwargs: extra attributes forwarded to the upstream java_library
        (deps, resources, data).
    """
    _java_wrap_library(name, srcs, visibility = visibility, **kwargs)

def java_binary(name, srcs = None, main_class = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `java_binary` (M23).

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
      **kwargs: extra attributes forwarded to the upstream java_binary.
    """
    effective_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)
    if main_class != None:
        upstream_kwargs["main_class"] = main_class
    _java_wrap_binary(name, effective_srcs, visibility = visibility, **upstream_kwargs)

def java_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `java_test` (M23).

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. Uses Bazel's
    standard test and coverage protocols.

    Args:
      name: public test target name (upstream target is name_upstream).
      srcs: direct test sources.
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream java_test
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
    _java_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    _java_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
    )
