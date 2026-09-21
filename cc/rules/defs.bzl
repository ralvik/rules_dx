"""Experimental minimal C/C++ wrappers (ADR 0019).

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
"""

load("@rules_cc//cc:defs.bzl", _cc_binary = "cc_binary", _cc_library = "cc_library", _cc_test = "cc_test")
load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo")

_CC_SRCS = [".c", ".cc", ".cpp", ".cxx"]
_CC_HDRS = [".h", ".hh", ".hpp", ".hxx"]

_DX_CC_LIBRARY_PROVIDES = [
    CcInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream `CcInfo`,
# `InstrumentedFilesInfo`, `OutputGroupInfo`, and `RunEnvironmentInfo`
# at runtime when present, but advertise only `DefaultInfo` plus
# `QualitySourcesInfo`: neither shape is depended on as a C++ library,
# so no consumer matches on the forwarded providers. `QualitySourcesInfo`
# is advertised so quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `go_*` test forwarder).
_DX_CC_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

# Header ownership is per extension, not per including source: `.h` maps to
# `c` and `.hh`/`.hpp`/`.hxx` map to `cpp`.
_DX_CC_SOURCE_SPECS = [
    ("c", ["c", "h"]),
    ("cpp", ["cc", "cpp", "cxx", "hh", "hpp", "hxx"]),
]

_cc_library_forward = dx_library_forward_rule(
    provides = _DX_CC_LIBRARY_PROVIDES,
    required_providers = [(CcInfo, "CcInfo")],
    quality_specs = _DX_CC_SOURCE_SPECS,
    what = "cc_*",
    allow_files = _CC_SRCS,
    upstream_providers = [[CcInfo]],
    doc = "Forwards upstream C++ library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct C/C++ sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream cc_library target whose providers are preserved.",
    extra_attrs = {
        "hdrs": attr.label_list(
            allow_files = _CC_HDRS,
            doc = "Direct C/C++ headers owned by this wrapper for QualitySourcesInfo.",
        ),
    },
    extra_quality_attrs = ["hdrs"],
)

_cc_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_CC_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_CC_SOURCE_SPECS,
    what = "cc_*",
    allow_files = _CC_SRCS,
    upstream_providers = [[CcInfo]],
    doc = "Executable forwarder for cc_binary: symlinks the upstream binary.",
    srcs_doc = "Direct C/C++ sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream cc_binary target whose executable is symlinked.",
    optional_providers = [CcInfo],
    runtime = "besteffort",
)

_cc_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_CC_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_CC_SOURCE_SPECS,
    what = "cc_*",
    allow_files = _CC_SRCS,
    upstream_providers = [[CcInfo]],
    doc = "Test forwarder for cc_test: symlinks the upstream test executable.",
    srcs_doc = "Direct C/C++ test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream cc_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [CcInfo],
)

def _cc_with_werror(kwargs):
    upstream_kwargs = dict(kwargs)
    copts = list(upstream_kwargs.get("copts", []))
    if "-Werror" not in copts:
        copts = copts + ["-Werror"]
    upstream_kwargs["copts"] = copts
    return upstream_kwargs

def _cc_wrap_library(name, srcs, hdrs, visibility = None, **kwargs):
    dx_wrap(name, _cc_library, _cc_library_forward, srcs, hdrs = hdrs, visibility = visibility, **_cc_with_werror(kwargs))

def _cc_wrap_binary(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _cc_binary, _cc_binary_forward, srcs, visibility = visibility, **_cc_with_werror(kwargs))

def cc_library(name, srcs = None, hdrs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `cc_library`."""
    effective_srcs = srcs if srcs != None else []
    effective_hdrs = hdrs if hdrs != None else []
    _cc_wrap_library(name, effective_srcs, effective_hdrs, visibility = visibility, **kwargs)

def cc_binary(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `cc_binary`.

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library.
    Headers arrive via the library `deps`, never as binary `hdrs`."""
    _cc_wrap_binary(name, srcs, visibility = visibility, **kwargs)

def cc_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `cc_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner
    via `deps`; tested sources are never this test's direct sources. Plain
    assert tests (exit code is the verdict) stay supported; the GoogleTest
 v1.18.0 mapping is qualified
    (`cc/tests/fixtures/googletest/`: `TEST()` plus `EXPECT_*` sources over
    the pinned `@googletest//:gtest_main` with an explicit `-std=c++17`
    floor; living at head rejected). Uses Bazel's standard test and
    coverage protocols."""
    test_srcs = srcs if srcs != None else []
    upstream_kwargs = _cc_with_werror(kwargs)

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
    _cc_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    _cc_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
        **({"aspect_hints": kwargs["aspect_hints"]} if "aspect_hints" in kwargs else {})
    )
