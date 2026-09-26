"""Experimental minimal Go wrappers (ADR 0019).

Upstream: rules_go 0.63.0 plus Go SDK 1.26.6 (MODULE.bazel).
"""

load("@rules_go//go:def.bzl", _GoArchive = "GoArchive", _GoInfo = "GoInfo", _go_binary = "go_binary", _go_library = "go_library", _go_test = "go_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap", "dx_wrap_binary", "dx_wrap_test")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_GO_LIBRARY_PROVIDES = [
    _GoInfo,
    _GoArchive,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_DX_GO_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_DX_GO_SOURCE_SPECS = [("go", "go")]
_DX_GO_SOURCE_EXTS = [".go"]

_go_library_forward = dx_library_forward_rule(
    provides = _DX_GO_LIBRARY_PROVIDES,
    required_providers = [(_GoInfo, "GoInfo"), (_GoArchive, "GoArchive")],
    quality_specs = _DX_GO_SOURCE_SPECS,
    what = "go_*",
    allow_files = _DX_GO_SOURCE_EXTS,
    upstream_providers = [[_GoInfo]],
    doc = "Forwards upstream Go library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Go sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream go_library target whose providers are preserved.",
)

_go_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_GO_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_GO_SOURCE_SPECS,
    what = "go_*",
    allow_files = _DX_GO_SOURCE_EXTS,
    upstream_providers = [[_GoArchive]],
    doc = "Executable forwarder for go_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Go sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream go_binary target whose executable is symlinked.",
    optional_providers = [_GoArchive],
    runtime = "besteffort",
)

_go_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_GO_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_GO_SOURCE_SPECS,
    what = "go_*",
    allow_files = _DX_GO_SOURCE_EXTS,
    upstream_providers = [[_GoArchive]],
    doc = "Test forwarder for go_test: symlinks the upstream test executable.",
    srcs_doc = "Direct Go test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream go_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [_GoArchive],
)

def go_effective_srcs(srcs):
    """Returns the effective direct sources for a go binary or test shape.

    None becomes empty.
    """
    return srcs if srcs != None else []

def _go_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _go_library, _go_library_forward, srcs, visibility = visibility, **kwargs)

def _go_wrap_binary(name, srcs, visibility = None, **kwargs):
    dx_wrap_binary(name, _go_binary, _go_binary_forward, srcs, visibility = visibility, **kwargs)

def go_library(name, srcs, importpath, visibility = None, **kwargs):
    """Experimental minimal wrapper over `go_library`.

    Handwritten wrappers may set upstream cgo scope attrs (`cgo`, `pure`,
    `race`, `gotags`, `cdeps`); generated rules never do (pure-Go scope).
    """
    _go_wrap_library(name, srcs, visibility = visibility, importpath = importpath, **kwargs)

def go_binary(name, srcs = None, importpath = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `go_binary`.

    Two shapes: an ordinary binary owns its `srcs` (package main plus
    `deps` on a wrapper library), while a thin entry binary generated
    for a recognized entry carries only `embed = [":<library>"]` with
    no `srcs`. The library alone owns the source and its
    source-derived dependencies in the thin shape; the thin binary
    reports no direct sources. Both shapes preserve the upstream
    providers and execution semantics.
    """
    effective_srcs = go_effective_srcs(srcs)
    if importpath != None:
        _go_wrap_binary(name, effective_srcs, visibility = visibility, importpath = importpath, **kwargs)
    else:
        _go_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def go_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `go_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary
    owner via `embed` (native package-level test semantics per the
    generation contract Go exception); embedded sources are never this
    test's direct sources. Uses `go test` and Bazel's standard test
    and coverage protocols. Handwritten tests may set upstream race
    scope (`race`, `pure`); generated tests never do."""
    dx_wrap_test(name, _go_test, _go_forward_test, srcs, visibility = visibility, **kwargs)
