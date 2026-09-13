"""Experimental minimal Go wrappers (M22, O30, ADR 0019).

Thin conventional boundary over the pinned `rules_go 0.63.0` ruleset
(Go SDK `1.26.6`, see MODULE.bazel). Each `dx_go_*` macro creates one
private `<name>_dx_upstream` target with the passed attributes and one
public `<name>` forwarding target. The library forwarder preserves the
upstream providers (`GoInfo`, `GoArchive`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `srcs`. Binaries and tests use an
executable forwarder whose own symlink action points at the upstream
executable (Bazel requires executable-providing rules to create the
file themselves).

Used upstream symbols (`@rules_go//go:def.bzl`): `go_library`,
`go_binary` (a macro over the inner executable rule), `go_test`,
`GoInfo`, `GoArchive`. No other upstream surface is used; consumers
needing more load the upstream module directly.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"go": <direct .go>})`. Import
paths, transitive archives, and module closures stay readable from the
preserved `GoInfo`/`GoArchive`; no second provider duplicates them.

`importpath` is always passed through with no invented default;
Gazelle owns importpath inference later. Wrappers accept no toolchain
version fields; unknown versions fail in upstream toolchain resolution,
never here. Only `.go` sources are accepted: cgo (`.c`/`.h`/assembly)
scope stays unresolved per the support-matrix feasibility review and
fails closed here until O30 qualifies it.
"""

load("@rules_go//go:def.bzl", _GoArchive = "GoArchive", _GoInfo = "GoInfo", _go_binary = "go_binary", _go_library = "go_library", _go_test = "go_test")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

_DX_GO_LIBRARY_PROVIDES = [
    _GoInfo,
    _GoArchive,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream `GoArchive`,
# `InstrumentedFilesInfo`, `OutputGroupInfo`, and `RunEnvironmentInfo`
# at runtime when present (mirroring upstream, which advertises only
# `GoArchive`), but advertise only `DefaultInfo` plus
# `QualitySourcesInfo`: neither shape is depended on as a Go library,
# so no consumer matches on the forwarded providers. `QualitySourcesInfo`
# is advertised so M22+ quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `dx_js_*` test forwarder).
_DX_GO_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

def _dx_go_quality_sources(ctx):
    go = [f for f in ctx.files.srcs if f.extension == "go"]
    direct_sources = {}
    if len(go) > 0:
        direct_sources["go"] = depset(go)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _dx_go_preserved_library_providers(ctx):
    upstream = ctx.attr.upstream
    if _GoInfo not in upstream:
        fail("dx_go_*: upstream target has no GoInfo: " + str(ctx.attr.upstream.label))
    if _GoArchive not in upstream:
        fail("dx_go_*: upstream target has no GoArchive: " + str(ctx.attr.upstream.label))
    return [upstream[_GoInfo], upstream[_GoArchive]]

def _dx_go_forwarded_output_providers(ctx):
    """Output groups and run env forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    out = []
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _dx_go_forwarded_archive(ctx):
    """Upstream `GoArchive` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if _GoArchive in upstream:
        return [upstream[_GoArchive]]
    return []

def _dx_go_forwarded_instrumented(ctx):
    """Upstream `InstrumentedFilesInfo` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo in upstream:
        return [upstream[InstrumentedFilesInfo]]
    return []

def _dx_go_library_forward_impl(ctx):
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo not in upstream:
        fail("dx_go_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    return (
        _dx_go_preserved_library_providers(ctx) +
        [upstream[DefaultInfo]] +
        [upstream[InstrumentedFilesInfo]] +
        _dx_go_forwarded_output_providers(ctx) +
        [_dx_go_quality_sources(ctx)]
    )

_dx_go_library_forward = rule(
    implementation = _dx_go_library_forward_impl,
    provides = _DX_GO_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".go"],
            doc = "Direct Go sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_GoInfo]],
            doc = "The private upstream go_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream Go library providers unchanged and adds QualitySourcesInfo.",
)

def _dx_go_symlink_default_info(ctx):
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail("dx_go_*: upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def _dx_go_binary_forward_impl(ctx):
    return (
        [_dx_go_symlink_default_info(ctx)] +
        _dx_go_forwarded_archive(ctx) +
        _dx_go_forwarded_instrumented(ctx) +
        _dx_go_forwarded_output_providers(ctx) +
        [_dx_go_quality_sources(ctx)]
    )

_dx_go_binary_forward = rule(
    implementation = _dx_go_binary_forward_impl,
    executable = True,
    provides = _DX_GO_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".go"],
            doc = "Direct Go sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_GoArchive]],
            doc = "The private upstream go_binary target whose executable is symlinked.",
        ),
    },
    doc = "Executable forwarder for dx_go_binary: symlinks the upstream binary.",
)

def _dx_go_test_forward_impl(ctx):
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo not in upstream:
        fail("dx_go_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    return (
        [_dx_go_symlink_default_info(ctx)] +
        [upstream[InstrumentedFilesInfo]] +
        _dx_go_forwarded_archive(ctx) +
        _dx_go_forwarded_output_providers(ctx) +
        [_dx_go_quality_sources(ctx)]
    )

_dx_go_forward_test = rule(
    implementation = _dx_go_test_forward_impl,
    test = True,
    provides = _DX_GO_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".go"],
            doc = "Direct Go test sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_GoArchive]],
            doc = "The private upstream go_test target whose executable is symlinked.",
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
    doc = "Test forwarder for dx_go_test: symlinks the upstream test executable.",
)

def _dx_go_wrap_library(name, srcs, visibility = None, **kwargs):
    _go_library(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_go_library_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def _dx_go_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = dict(kwargs)
    if len(srcs) > 0:
        upstream_kwargs["srcs"] = srcs
    _go_binary(
        name = name + "_dx_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    _dx_go_binary_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def dx_go_library(name, srcs, importpath, visibility = None, **kwargs):
    """Experimental minimal wrapper over `go_library` (M22).

    Args:
      name: public library target name (upstream target is name_dx_upstream).
      srcs: direct Go sources owned by this wrapper.
      importpath: library import path, passed through with no default.
      visibility: visibility of the public forwarding library target.
      **kwargs: extra attributes forwarded to the upstream go_library
        (deps, embed, data, importmap).
    """
    _dx_go_wrap_library(name, srcs, visibility = visibility, importpath = importpath, **kwargs)

def dx_go_binary(name, srcs = None, importpath = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `go_binary` (M22).

    Two shapes: an ordinary binary owns its `srcs` (package main plus
    `deps` on a wrapper library), while a thin entry binary generated
    for a recognized entry carries only `embed = [":<library>"]` with
    no `srcs`. The library alone owns the source and its
    source-derived dependencies in the thin shape; the thin binary
    reports no direct sources. Both shapes preserve the upstream
    providers and execution semantics.

    Args:
      name: public binary target name (upstream target is name_dx_upstream).
      srcs: direct binary sources; empty for thin entry binaries.
      importpath: binary import path; none for thin entry binaries
        (inferred from the embedded library upstream).
      visibility: visibility of the public forwarding binary target.
      **kwargs: extra attributes forwarded to the upstream go_binary.
    """
    effective_srcs = srcs if srcs != None else []
    if importpath != None:
        _dx_go_wrap_binary(name, effective_srcs, visibility = visibility, importpath = importpath, **kwargs)
    else:
        _dx_go_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def dx_go_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `go_test` (M22).

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary
    owner via `embed` (native package-level test semantics per the
    generation contract Go exception); embedded sources are never this
    test's direct sources. Uses `go test` and Bazel's standard test
    and coverage protocols.

    Args:
      name: public test target name (upstream target is name_dx_upstream).
      srcs: direct test sources (for example `*_test.go` files).
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream go_test
        (embed must name the wrapper library under test).
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
    _go_test(
        name = name + "_dx_upstream",
        **upstream_kwargs
    )
    _dx_go_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_dx_upstream",
        srcs = test_srcs,
        visibility = visibility,
    )
