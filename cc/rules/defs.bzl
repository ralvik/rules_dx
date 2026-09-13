"""Experimental minimal C/C++ wrappers (M22, O30, ADR 0019).

Thin conventional boundary over the pinned `rules_cc 0.2.22` ruleset
(which proxies the native `cc_library`/`cc_binary`/`cc_test`). Each
`dx_cc_*` macro creates one private `<name>_dx_upstream` target with the
passed attributes and one public `<name>` forwarding target. The library
forwarder preserves the upstream providers (`CcInfo`, `DefaultInfo`,
`InstrumentedFilesInfo`) unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `srcs`/`hdrs`. Binaries and tests use
an executable forwarder whose own symlink action points at the upstream
executable (Bazel requires executable-providing rules to create the file
themselves).

Used upstream symbols (`@rules_cc//cc:defs.bzl`): `cc_library`,
`cc_binary`, `cc_test`. Used provider (`@rules_cc//cc/common:cc_info.bzl`):
`CcInfo`. No other upstream surface is used; consumers needing more
(`cc_import`, `cc_shared_library`, `objc_*`) load the upstream module
directly.

Normalization is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"c": <direct .c/.h>,
"cpp": <direct .cc/.cpp/.cxx/.hh/.hpp/.hxx>})`. Include paths,
transitive headers, and toolchain closures stay readable from the
preserved `CcInfo`; no second provider duplicates them.

Header ownership is per extension, not per including source: `.h` maps to
`c` and `.hh`/`.hpp`/`.hxx` map to `cpp`. A C++ library with a `.h`
header therefore reports both classes under the single `cc` family; the
pipeline groups them by family, so no second owner is created. CUDA
(`.cu`/`.cuh`), assembly, and `cc_shared_library` scope stay unresolved
per the support-matrix feasibility review and fail closed here until O30
qualifies them.
"""

load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load("@rules_cc//cc:defs.bzl", _cc_binary = "cc_binary", _cc_library = "cc_library", _cc_test = "cc_test")
load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

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
# is advertised so M22+ quality aspects can gate on it. Coverage reads
# `InstrumentedFilesInfo` from the test target, not via `provides`
# (same shape as the `dx_go_*` test forwarder).
_DX_CC_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

def _dx_cc_split_sources(files):
    c = [f for f in files if f.extension in ("c", "h")]
    cpp = [f for f in files if f.extension in ("cc", "cpp", "cxx", "hh", "hpp", "hxx")]
    return c, cpp

def _dx_cc_quality_sources(ctx):
    files = list(ctx.files.srcs)
    if hasattr(ctx.files, "hdrs"):
        files.extend(ctx.files.hdrs)
    c, cpp = _dx_cc_split_sources(files)
    direct_sources = {}
    if len(c) > 0:
        direct_sources["c"] = depset(c)
    if len(cpp) > 0:
        direct_sources["cpp"] = depset(cpp)
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _dx_cc_preserved_library_providers(ctx):
    upstream = ctx.attr.upstream
    if CcInfo not in upstream:
        fail("dx_cc_*: upstream target has no CcInfo: " + str(ctx.attr.upstream.label))
    return [upstream[CcInfo]]

def _dx_cc_forwarded_output_providers(ctx):
    """Output groups and run env forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    out = []
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _dx_cc_forwarded_cc(ctx):
    """Upstream `CcInfo` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if CcInfo in upstream:
        return [upstream[CcInfo]]
    return []

def _dx_cc_forwarded_instrumented(ctx):
    """Upstream `InstrumentedFilesInfo` forwarded best-effort when present."""
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo in upstream:
        return [upstream[InstrumentedFilesInfo]]
    return []

def _dx_cc_library_forward_impl(ctx):
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo not in upstream:
        fail("dx_cc_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    return (
        _dx_cc_preserved_library_providers(ctx) +
        [upstream[DefaultInfo]] +
        [upstream[InstrumentedFilesInfo]] +
        _dx_cc_forwarded_output_providers(ctx) +
        [_dx_cc_quality_sources(ctx)]
    )

_dx_cc_library_forward = rule(
    implementation = _dx_cc_library_forward_impl,
    provides = _DX_CC_LIBRARY_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = _CC_SRCS,
            doc = "Direct C/C++ sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "hdrs": attr.label_list(
            allow_files = _CC_HDRS,
            doc = "Direct C/C++ headers owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[CcInfo]],
            doc = "The private upstream cc_library target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream C++ library providers unchanged and adds QualitySourcesInfo.",
)

def _dx_cc_symlink_default_info(ctx):
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail("dx_cc_*: upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def _dx_cc_binary_forward_impl(ctx):
    return (
        [_dx_cc_symlink_default_info(ctx)] +
        _dx_cc_forwarded_cc(ctx) +
        _dx_cc_forwarded_instrumented(ctx) +
        _dx_cc_forwarded_output_providers(ctx) +
        [_dx_cc_quality_sources(ctx)]
    )

_dx_cc_binary_forward = rule(
    implementation = _dx_cc_binary_forward_impl,
    executable = True,
    provides = _DX_CC_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = _CC_SRCS,
            doc = "Direct C/C++ sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[CcInfo]],
            doc = "The private upstream cc_binary target whose executable is symlinked.",
        ),
    },
    doc = "Executable forwarder for dx_cc_binary: symlinks the upstream binary.",
)

def _dx_cc_test_forward_impl(ctx):
    upstream = ctx.attr.upstream
    if InstrumentedFilesInfo not in upstream:
        fail("dx_cc_*: upstream target has no InstrumentedFilesInfo: " + str(ctx.attr.upstream.label))
    return (
        [_dx_cc_symlink_default_info(ctx)] +
        [upstream[InstrumentedFilesInfo]] +
        _dx_cc_forwarded_cc(ctx) +
        _dx_cc_forwarded_output_providers(ctx) +
        [_dx_cc_quality_sources(ctx)]
    )

_dx_cc_forward_test = rule(
    implementation = _dx_cc_test_forward_impl,
    test = True,
    provides = _DX_CC_EXEC_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = _CC_SRCS,
            doc = "Direct C/C++ test sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[CcInfo]],
            doc = "The private upstream cc_test target whose executable is symlinked.",
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
    doc = "Test forwarder for dx_cc_test: symlinks the upstream test executable.",
)

def _dx_cc_wrap_library(name, srcs, hdrs, visibility = None, **kwargs):
    _cc_library(
        name = name + "_dx_upstream",
        srcs = srcs,
        hdrs = hdrs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_cc_library_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        hdrs = hdrs,
        visibility = visibility,
    )

def _dx_cc_wrap_binary(name, srcs, visibility = None, **kwargs):
    _cc_binary(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    _dx_cc_binary_forward(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        visibility = visibility,
    )

def dx_cc_library(name, srcs = None, hdrs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `cc_library` (M22).

    Args:
      name: public library target name (upstream target is name_dx_upstream).
      srcs: direct C/C++ sources owned by this wrapper.
      hdrs: direct C/C++ headers owned by this wrapper.
      visibility: visibility of the public forwarding library target.
      **kwargs: extra attributes forwarded to the upstream cc_library
        (deps, includes, copts, defines).
    """
    effective_srcs = srcs if srcs != None else []
    effective_hdrs = hdrs if hdrs != None else []
    _dx_cc_wrap_library(name, effective_srcs, effective_hdrs, visibility = visibility, **kwargs)

def dx_cc_binary(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `cc_binary` (M22).

    An ordinary binary owns its `srcs` plus `deps` on a wrapper library.
    Headers arrive via the library `deps`, never as binary `hdrs`.

    Args:
      name: public binary target name (upstream target is name_dx_upstream).
      srcs: direct binary sources owned by this wrapper.
      visibility: visibility of the public forwarding binary target.
      **kwargs: extra attributes forwarded to the upstream cc_binary.
    """
    _dx_cc_wrap_binary(name, srcs, visibility = visibility, **kwargs)

def dx_cc_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `cc_test` (M22).

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner
    via `deps`; tested sources are never this test's direct sources. Uses
    Bazel's standard test and coverage protocols.

    Args:
      name: public test target name (upstream target is name_dx_upstream).
      srcs: direct test sources owned by this wrapper.
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream cc_test
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
    _cc_test(
        name = name + "_dx_upstream",
        **upstream_kwargs
    )
    _dx_cc_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_dx_upstream",
        srcs = test_srcs,
        visibility = visibility,
    )
