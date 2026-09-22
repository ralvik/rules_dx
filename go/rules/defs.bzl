"""Experimental minimal Go wrappers (ADR 0019).

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
Upstream: rules_go 0.63.0 plus Go SDK 1.26.6 (MODULE.bazel).
"""

load("@rules_go//go:def.bzl", _GoArchive = "GoArchive", _GoInfo = "GoInfo", _go_binary = "go_binary", _go_library = "go_library", _go_test = "go_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_forwarded_test_kwargs", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

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
# is advertised so quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `javascript_*` test forwarder).
_DX_GO_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

# Go owns `.go` only. `go_module` (`go.mod`/`go.sum`) stays fixture-owned
# ad-hoc (See: docs/quality/quality-sources.md): upstream `go_*` rules take
# no manifest `srcs`, so `modfmt` runs via matrix fixtures, never these wrappers.
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

def _go_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _go_library, _go_library_forward, srcs, visibility = visibility, **kwargs)

def _go_wrap_binary(name, srcs, visibility = None, **kwargs):
    upstream_kwargs = dict(kwargs)
    if len(srcs) > 0:
        upstream_kwargs["srcs"] = srcs
    _go_binary(
        name = name + "_upstream",
        visibility = ["//visibility:private"],
        **upstream_kwargs
    )
    forward_kwargs = {}
    if "tags" in kwargs:
        forward_kwargs["tags"] = kwargs["tags"]
    if "aspect_hints" in kwargs:
        forward_kwargs["aspect_hints"] = kwargs["aspect_hints"]
    _go_binary_forward(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
        **forward_kwargs
    )

def go_library(name, srcs, importpath, visibility = None, **kwargs):
    """Experimental minimal wrapper over `go_library`.

    Handwritten wrappers may set upstream cgo scope attrs (`cgo`, `pure`,
    `race`, `gotags`, `cdeps`); generated rules never do (pure-Go scope)."""
    _go_wrap_library(name, srcs, visibility = visibility, importpath = importpath, **kwargs)

def go_binary(name, srcs = None, importpath = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `go_binary`.

    Two shapes: an ordinary binary owns its `srcs` (package main plus
    `deps` on a wrapper library), while a thin entry binary generated
    for a recognized entry carries only `embed = [":<library>"]` with
    no `srcs`. The library alone owns the source and its
    source-derived dependencies in the thin shape; the thin binary
    reports no direct sources. Both shapes preserve the upstream
    providers and execution semantics."""
    effective_srcs = srcs if srcs != None else []
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
    test_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)

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
    _go_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    forward_kwargs = dx_forwarded_test_kwargs(kwargs)
    if "aspect_hints" in kwargs:
        forward_kwargs["aspect_hints"] = kwargs["aspect_hints"]
    _go_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
        **forward_kwargs
    )
