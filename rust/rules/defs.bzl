"""Experimental minimal Rust wrappers (M02, ADR 0013).

Thin conventional boundary over the pinned `rules_rust 0.74.0` ruleset.
Each `dx_rust_*` macro creates one private `<name>_dx_upstream` target with
the passed compilation attributes and one public `<name>` forwarding
target. The forwarder preserves the upstream providers (`CrateInfo` or
`TestCrateInfo`, `DepInfo`, `DefaultInfo`, `OutputGroupInfo`) unchanged
and adds `QualitySourcesInfo` normalized from the wrapper's direct `srcs`.
Libraries forward `DefaultInfo` wholesale; binaries and tests use an
executable forwarder whose own symlink action points at the upstream
executable (Bazel requires executable-providing rules to create the file
themselves).

Used upstream symbols (`@rules_rust//rust:defs.bzl`): `rust_library`,
`rust_binary`, `rust_test`, `rust_proc_macro`, `rust_shared_library`,
`rust_static_library`, `rust_common` (`crate_info`, `dep_info`,
`test_crate_info`), `rustfmt_test`, `rustfmt_aspect`, `rust_clippy_test`,
`rust_clippy_aspect`. Used toolchain types:
`@rules_rust//rust:toolchain_type` (`rustc`, `cargo`, `rustfmt`,
`clippy_driver`), `@rules_rust//rust/rustfmt:toolchain_type`
(`rustfmt`). No other upstream surface is used; consumers needing more
load the upstream module directly.

Normalization (WP3) is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"rust": <direct srcs>})`. Crate
name, edition, root, and sources for generation work stay readable from
the preserved `CrateInfo`; no second provider duplicates them. A test
using `crate =` reports no direct sources: the crate owner is the single
source owner, never the test.

`edition` defaults to `"2021"`, matching the pinned toolchain default;
override per target. Unknown or unregistered toolchain versions fail in
the upstream toolchain resolution, never here.

Each forwarder advertises `provides = [CrateInfo, DepInfo, DefaultInfo,
InstrumentedFilesInfo, QualitySourcesInfo]`: Bazel matches an aspect's
`required_providers`
against advertised providers, so without `provides` the upstream lint
aspects skip the wrappers and the lint tests pass vacuously. `provides`
is strictly validated (advertised implies returned), so the forwarder
fails loudly when the private upstream lacks `CrateInfo`/`DepInfo`
instead of silently changing shape. `OutputGroupInfo` and
`RunEnvironmentInfo` are forwarded at runtime when present but stay
unadvertised, mirroring upstream: unadvertised providers remain visible
to Starlark reads and `--output_groups`. `QualitySourcesInfo` is
advertised so M04 quality aspects can gate on it.
"""

load("@rules_rust//rust:defs.bzl", _rust_binary = "rust_binary", _rust_common = "rust_common", _rust_library = "rust_library", _rust_proc_macro = "rust_proc_macro", _rust_shared_library = "rust_shared_library", _rust_static_library = "rust_static_library", _rust_test = "rust_test")
load("//quality:sources.bzl", "QualitySourcesInfo", "RUST", "check_direct_sources")

_DEFAULT_EDITION = "2021"

# Advertised providers. Bazel matches an aspect's `required_providers`
# against the rule's `provides`, NOT against the providers the rule
# implementation actually returns (verified by experiment: a forwarder that
# returns `CrateInfo` without advertising it is silently skipped by
# `rustfmt_aspect`/`rust_clippy_aspect`, making `rustfmt_test` vacuous).
# `provides` is also strictly validated: every advertised provider must be
# returned. Advertised sets therefore contain exactly the always-returned
# providers: `CrateInfo` and `DepInfo` (every wrapped upstream rule yields
# both per `rustc.bzl`), `DefaultInfo` (wholesale or self-made symlink),
# and `QualitySourcesInfo` so M04 quality aspects can gate on it.
# `OutputGroupInfo` and `RunEnvironmentInfo` stay unadvertised but are
# forwarded at runtime when present: unadvertised providers remain visible
# to Starlark reads and `--output_groups` (mirroring upstream, which
# advertises only `COMMON_PROVIDERS` yet returns output groups).
# `InstrumentedFilesInfo` IS advertised and always forwarded: Bazel only
# sets up coverage collection for tests that provide it, and the forwarded
# object keeps the test's instrumented sources identical to upstream's.
# Coverage additionally needs the `_lcov_merger` magic attribute on the
# test forwarder (see below): without it `collect_coverage.sh` exits after
# touching an empty `coverage.dat` even though the staging report is full.
_DX_FORWARD_PROVIDES = [
    _rust_common.crate_info,
    _rust_common.dep_info,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

def _dx_quality_sources(ctx):
    direct_sources = {}
    direct = depset(ctx.files.srcs)
    if len(direct.to_list()) != 0:
        direct_sources[RUST] = direct
    check_direct_sources(direct_sources, str(ctx.label))
    return QualitySourcesInfo(direct_sources = direct_sources)

def _dx_preserved_crate_providers(ctx):
    """`CrateInfo` + `DepInfo` from the private upstream; both are mandatory.

    Every upstream rule the macros create (`rust_library`, `rust_binary`,
    `rust_test`) yields both. Anything else is a wrapper bug, so fail loudly
    instead of silently changing shape (which would also break the
    advertised `provides` contract above).
    """
    upstream = ctx.attr.upstream
    if _rust_common.crate_info not in upstream:
        fail("dx_rust_*: upstream target has no CrateInfo: " +
             str(ctx.attr.upstream.label))
    if _rust_common.dep_info not in upstream:
        fail("dx_rust_*: upstream target has no DepInfo: " +
             str(ctx.attr.upstream.label))
    return [upstream[_rust_common.crate_info], upstream[_rust_common.dep_info]]

def _dx_forwarded_runtime_providers(ctx):
    """Runtime fidelity: coverage metadata, output groups, and test/run env.

    `InstrumentedFilesInfo` is mandatory: Bazel only collects coverage for
    tests that provide it, so dropping it would silently empty the wrapper
    test's `coverage.dat` and fail the implementation-coverage gate. Output
    groups and run env stay best-effort: they are absent on some upstream
    shapes and their absence changes nothing observable.
    """
    upstream = ctx.attr.upstream
    out = []
    if InstrumentedFilesInfo not in upstream:
        fail("dx_rust_*: upstream target has no InstrumentedFilesInfo: " +
             str(ctx.attr.upstream.label))
    out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def _dx_rust_forward_impl(ctx):
    return (
        _dx_preserved_crate_providers(ctx) +
        [ctx.attr.upstream[DefaultInfo]] +
        _dx_forwarded_runtime_providers(ctx) +
        [_dx_quality_sources(ctx)]
    )

_dx_rust_forward = rule(
    implementation = _dx_rust_forward_impl,
    provides = _DX_FORWARD_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".rs"],
            doc = "Direct Rust sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [
                [_rust_common.crate_info],
                [_rust_common.test_crate_info],
            ],
            doc = "The private upstream rust_* target whose providers are preserved.",
        ),
    },
    doc = "Forwards upstream Rust providers unchanged and adds QualitySourcesInfo.",
)

def _dx_rust_forwarded_non_default_providers(ctx):
    """Preserved upstream providers plus QualitySourcesInfo, minus DefaultInfo."""
    return (
        _dx_preserved_crate_providers(ctx) +
        _dx_forwarded_runtime_providers(ctx) +
        [_dx_quality_sources(ctx)]
    )

def _dx_rust_symlink_default_info(ctx):
    # A rule that provides an executable must create that file itself, so
    # the forwarder cannot pass the upstream DefaultInfo through. A symlink
    # created by this rule's own action satisfies the check while keeping
    # run/test behavior identical to the upstream target (verified in
    # scratch: `bazel run` output and `bazel test` status match).
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail("dx_rust_*: upstream target has no executable: " +
             str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def _dx_rust_forward_binary_impl(ctx):
    return [_dx_rust_symlink_default_info(ctx)] + _dx_rust_forwarded_non_default_providers(ctx)

_dx_rust_forward_binary = rule(
    implementation = _dx_rust_forward_binary_impl,
    executable = True,
    provides = _DX_FORWARD_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".rs"],
            doc = "Direct Rust sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [
                [_rust_common.crate_info],
                [_rust_common.test_crate_info],
            ],
            doc = "The private upstream rust_binary target whose providers are preserved.",
        ),
    },
    doc = "Executable forwarder for dx_rust_binary: symlinks the upstream binary.",
)

def _dx_rust_forward_test_impl(ctx):
    return [_dx_rust_symlink_default_info(ctx)] + _dx_rust_forwarded_non_default_providers(ctx)

_dx_rust_forward_test = rule(
    implementation = _dx_rust_forward_test_impl,
    test = True,
    provides = _DX_FORWARD_PROVIDES,
    attrs = {
        "srcs": attr.label_list(
            allow_files = [".rs"],
            doc = "Direct Rust sources owned by this wrapper for QualitySourcesInfo.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [
                [_rust_common.crate_info],
                [_rust_common.test_crate_info],
            ],
            doc = "The private upstream rust_test target whose providers are preserved.",
        ),
        "_lcov_merger": attr.label(
            default = configuration_field(fragment = "coverage", name = "output_generator"),
            executable = True,
            cfg = "exec",
            doc = "Coverage-report merger. Bazel's coverage runner passes " +
                  "this magic attribute as LCOV_MERGER, which merges the " +
                  "per-test staging report into coverage.dat; without it " +
                  "the runner exits after touching an empty file even " +
                  "though the test collected coverage (see " +
                  "collect_coverage.sh). Same declaration as upstream " +
                  "rust_test.",
        ),
    },
    doc = "Test forwarder for dx_rust_test: symlinks the upstream test executable.",
)

def _dx_wrap(name, upstream_rule, forward_rule, srcs, visibility = None, testonly = False, **kwargs):
    # The private target keeps the wrapper's crate name: upstream derives
    # crate names from target names, and dots are invalid there. It stays
    # package-private: only the public forwarder may depend on it.
    kwargs.setdefault("crate_name", name)
    upstream_rule(
        name = name + "_dx_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    forward_rule(
        name = name,
        upstream = name + "_dx_upstream",
        srcs = srcs,
        testonly = testonly,
        visibility = visibility,
    )

def dx_rust_library(
        name,
        srcs,
        crate_name = None,
        edition = _DEFAULT_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_library` (M02)."""
    _dx_wrap(
        name,
        _rust_library,
        _dx_rust_forward,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def dx_rust_binary(
        name,
        srcs,
        crate_name = None,
        edition = _DEFAULT_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_binary` (M02)."""
    _dx_wrap(
        name,
        _rust_binary,
        _dx_rust_forward_binary,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def dx_rust_test(
        name,
        srcs = None,
        crate = None,
        edition = _DEFAULT_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_test` (M02).

    With `crate`, the referenced wrapper stays the single source owner and
    this target reports no direct sources. With `srcs`, those sources are
    this test's direct sources.

    Args:
      name: public test target name (upstream target is name_dx_upstream).
      srcs: direct test sources; none when testing via `crate`.
      crate: wrapper library target owning the sources under test.
      edition: Rust edition forwarded upstream.
      visibility: visibility of the public forwarding test target.
      **kwargs: extra attributes forwarded to the upstream rust_test.
    """
    test_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)
    upstream_kwargs["crate"] = crate
    upstream_kwargs["edition"] = edition
    upstream_kwargs.setdefault("crate_name", name)

    # The private test is an implementation detail: tag it manual so
    # `bazel test //...` exercises the public wrapper target only.
    upstream_kwargs.setdefault("tags", ["manual"])
    upstream_kwargs["visibility"] = ["//visibility:private"]
    if srcs != None:
        upstream_kwargs["srcs"] = srcs
    _rust_test(
        name = name + "_dx_upstream",
        **upstream_kwargs
    )
    _dx_rust_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_dx_upstream",
        srcs = test_srcs,
        visibility = visibility,
    )

def dx_rust_proc_macro(
        name,
        srcs,
        crate_name = None,
        edition = _DEFAULT_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_proc_macro` (M12).

    Same forwarding shape as `dx_rust_library`: the private upstream keeps
    the crate providers and the public target adds QualitySourcesInfo.
    """
    _dx_wrap(
        name,
        _rust_proc_macro,
        _dx_rust_forward,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def dx_rust_shared_library(
        name,
        srcs,
        crate_name = None,
        edition = _DEFAULT_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_shared_library` (M12).

    Same forwarding shape as `dx_rust_library`.
    """
    _dx_wrap(
        name,
        _rust_shared_library,
        _dx_rust_forward,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def dx_rust_static_library(
        name,
        srcs,
        crate_name = None,
        edition = _DEFAULT_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_static_library` (M12).

    Same forwarding shape as `dx_rust_library`.
    """
    _dx_wrap(
        name,
        _rust_static_library,
        _dx_rust_forward,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )
