"""Experimental minimal Scala wrappers (M23, O31, ADR 0019).

Thin conventional boundary over the pinned `rules_scala 7.3.0` ruleset,
managed route frozen by the M22 O30 decision and delivered here under O31.
Each `scala_*` macro creates one private `<name>_upstream` target with
the passed attributes and one public `<name>` forwarding target. The library
forwarder preserves the upstream providers (`JavaInfo`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `srcs`. Binaries and tests use an
executable forwarder whose own symlink action points at the upstream
executable (Bazel requires executable-providing rules to create the
file themselves).

Used upstream symbols (`@rules_scala//scala:scala.bzl`): `scala_library`,
`scala_binary`, `scala_test`. The preserved provider (`JavaInfo`) is
Bazel's native Java provider, which `scala_*` rules also provide alongside
their Scala-internal `ScalaInfo` (kept readable from the private upstream
target, never duplicated here). No other upstream surface is used;
consumers needing more (`scala_import`, `scala_junit_test`,
`scala_specs2_junit_test`, compiler plugins) load the upstream module
directly.

Normalization is deliberately narrow: direct `.scala` sources map to the
`scala` class and direct `.java` sources (same-compilation-unit mixed
sources) map to the `java` class:
`QualitySourcesInfo(direct_sources = {"scala": <direct .scala>,
"java": <direct .java>})`. Empty classes are omitted. Classpaths,
transitive jars, and toolchain closures stay readable from the preserved
`JavaInfo`; no second provider duplicates them.

`main_class` (binaries) always passes through with no invented default.
Wrappers accept no toolchain version fields; unknown versions fail in
upstream toolchain resolution, never here. The default Scala toolchain is
2.13.18 (pinned in MODULE.bazel via `scala_config.settings`); per-target
`scala_version` selection stays open under O31. Coursier-fetched toolchains
share Java's Maven-lock story (`maven_install.json` plus
`fail_if_repin_required`); Scalafix semantic rules need semanticdb plus
classpath wiring (open under O31).
"""

load("@rules_java//java:defs.bzl", "JavaInfo")
load("@rules_scala//scala:scala.bzl", _scala_binary = "scala_binary", _scala_library = "scala_library", _scala_test = "scala_test")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

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

def _scala_quality_sources(ctx):
    scala = [f for f in ctx.files.srcs if f.extension == "scala"]
    java = [f for f in ctx.files.srcs if f.extension == "java"]
    direct_sources = {}
    if len(scala) > 0:
        direct_sources["scala"] = depset(scala)
    if len(java) > 0:
        direct_sources["java"] = depset(java)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _scala_preserved_library_providers(ctx):
    upstream = ctx.attr.upstream
    if JavaInfo not in upstream:
        fail("scala_*: upstream target has no JavaInfo: " + str(ctx.attr.upstream.label))
    return [upstream[JavaInfo]]

def _scala_forwarded_output_providers(ctx):
    """Output groups and run env forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    out = []
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _scala_forwarded_java_info(ctx):
    """Upstream `JavaInfo` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if JavaInfo in upstream:
        return [upstream[JavaInfo]]
    return []

def _scala_forwarded_instrumented(ctx):
    """Upstream `InstrumentedFilesInfo` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo in upstream:
        return [upstream[InstrumentedFilesInfo]]
    return []

def _scala_library_forward_impl(ctx):
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo not in upstream:
        fail("scala_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    return (
        _scala_preserved_library_providers(ctx) +
        [upstream[DefaultInfo]] +
        [upstream[InstrumentedFilesInfo]] +
        _scala_forwarded_output_providers(ctx) +
        [_scala_quality_sources(ctx)]
    )

_scala_library_forward = rule(
    implementation = _scala_library_forward_impl,
    provides = _DX_SCALA_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".scala", ".java"],
            doc = "Direct Scala/Java sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[JavaInfo]],
            doc = "The private upstream scala_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream Scala library providers unchanged and adds QualitySourcesInfo.",
)

def _scala_symlink_default_info(ctx):
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail("scala_*: upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def _scala_binary_forward_impl(ctx):
    return (
        [_scala_symlink_default_info(ctx)] +
        _scala_forwarded_java_info(ctx) +
        _scala_forwarded_instrumented(ctx) +
        _scala_forwarded_output_providers(ctx) +
        [_scala_quality_sources(ctx)]
    )

_scala_binary_forward = rule(
    implementation = _scala_binary_forward_impl,
    executable = True,
    provides = _DX_SCALA_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".scala", ".java"],
            doc = "Direct Scala/Java sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            doc = "The private upstream scala_binary target whose executable is symlinked.",
        ),
    },
    doc = "Executable forwarder for scala_binary: symlinks the upstream binary.",
)

def _scala_test_forward_impl(ctx):
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo not in upstream:
        fail("scala_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    return (
        [_scala_symlink_default_info(ctx)] +
        [upstream[InstrumentedFilesInfo]] +
        _scala_forwarded_java_info(ctx) +
        _scala_forwarded_output_providers(ctx) +
        [_scala_quality_sources(ctx)]
    )

_scala_forward_test = rule(
    implementation = _scala_test_forward_impl,
    test = True,
    provides = _DX_SCALA_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".scala", ".java"],
            doc = "Direct Scala/Java test sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            doc = "The private upstream scala_test target whose executable is symlinked.",
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
    doc = "Test forwarder for scala_test: symlinks the upstream test executable.",
)

def _scala_wrap_library(name, srcs, visibility = None, **kwargs):
    _scala_library(
        name = name + "_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _scala_library_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
    )

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
    )

def scala_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `scala_library` (M23).

    Args:
      name: public library target name (upstream target is name_upstream).
      srcs: direct Scala/Java sources owned by this wrapper.
      visibility: visibility of the public forwarding library target.
      **kwargs: extra attributes forwarded to the upstream scala_library
        (deps, resources, data).
    """
    _scala_wrap_library(name, srcs, visibility = visibility, **kwargs)

def scala_binary(name, srcs = None, main_class = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `scala_binary` (M23).

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
      **kwargs: extra attributes forwarded to the upstream scala_binary.
    """
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
    toolchain (no Maven lock members needed for the hello closure).

    Args:
      name: public test target name (upstream target is name_upstream).
      srcs: direct test sources.
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream scala_test
        (deps must name the wrapper library under test).
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
    )
