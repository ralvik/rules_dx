"""Experimental minimal Rust wrappers (M02, ADR 0013).

Thin conventional boundary over the pinned `rules_rust 0.74.0` ruleset.
Each `rust_*` macro creates one private `<name>_upstream` target with
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
(`rustfmt`). `CcInfo` loads from `@rules_cc//cc/common:cc_info.bzl` for
the shared/static linking surface. No other upstream surface is used; consumers needing more
load the upstream module directly. `dx_rust_crate` additionally loads
`aliases` / `crate_deps` from the generated `@crates//:crates.bzl` and
`real_source_target` from `//quality:fixtures.bzl` to emit the full
M02 leaf-crate pattern (lib + test + lint tests + manifest + corpus);
neither target loads this module back, so the load graph stays acyclic.

Normalization (WP3) is deliberately narrow: the only new fact is
`QualitySourcesInfo(direct_sources = {"rust": <direct srcs>})`. Crate
name, edition, root, and sources for generation work stay readable from
the preserved `CrateInfo`; no second provider duplicates them. A test
using `crate =` reports no direct sources: the crate owner is the single
source owner, never the test.

`edition` defaults to `RUST_EDITION`, the single source of truth for the
repository Rust edition (issue #82): wrapper consumers omit `edition` and
inherit it, so an edition bump edits this constant only. It matches the
pinned toolchain default; override per target. Per-target toolchain-version selection is omitted under
ADR 0012: the pinned upstream only accepts a single toolchain version, so
there is no supported version-selectable API to map. Unknown or unregistered
toolchain versions fail in the upstream toolchain resolution, never here.

Each crate forwarder advertises `provides = [CrateInfo, DepInfo, DefaultInfo,
InstrumentedFilesInfo, QualitySourcesInfo]`: Bazel matches an aspect's
`required_providers`
against advertised providers, so without `provides` the upstream lint
aspects skip the wrappers and the lint tests pass vacuously. The
Cc-linking forwarders (`rust_shared_library`, `rust_static_library`)
advertise the same set with `TestCrateInfo` in place of `CrateInfo` plus
`CcInfo`: upstream provides no `CrateInfo` for those shapes, and the
advertised `TestCrateInfo` is what keeps the lint aspects matching.
`provides`
is strictly validated (advertised implies returned), so the forwarder
fails loudly when the private upstream lacks the advertised shape
instead of silently changing shape. `OutputGroupInfo` and
`RunEnvironmentInfo` are forwarded at runtime when present but stay
unadvertised, mirroring upstream: unadvertised providers remain visible
to Starlark reads and `--output_groups`. `QualitySourcesInfo` is
advertised so M04 quality aspects can gate on it.
"""

load("@crates//:crates.bzl", _aliases = "aliases", _crate_deps = "crate_deps")
load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load("@rules_rust//rust:defs.bzl", _rust_binary = "rust_binary", _rust_clippy_test = "rust_clippy_test", _rust_common = "rust_common", _rust_library = "rust_library", _rust_proc_macro = "rust_proc_macro", _rust_shared_library = "rust_shared_library", _rust_static_library = "rust_static_library", _rust_test = "rust_test", _rustfmt_test = "rustfmt_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap")
load("//quality:fixtures.bzl", "real_source_target")
load("//quality:sources.bzl", "QualitySourcesInfo", "RUST")

# Single source of truth for the repository Rust edition (issue #82).
# All wrapper macros default to this; BUILD files must not repeat the
# literal (omit `edition` on wrapper calls, load `RUST_EDITION` for the
# rare direct-upstream call).
RUST_EDITION = "2021"

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

_DX_RUST_SOURCE_SPECS = [(RUST, "rs")]
_DX_RUST_SOURCE_EXTS = [".rs"]
_DX_RUST_CRATE_PROVIDERS = [(_rust_common.crate_info, "CrateInfo"), (_rust_common.dep_info, "DepInfo")]

_rust_forward = dx_library_forward_rule(
    provides = _DX_FORWARD_PROVIDES,
    required_providers = _DX_RUST_CRATE_PROVIDERS,
    quality_specs = _DX_RUST_SOURCE_SPECS,
    what = "rust_*",
    allow_files = _DX_RUST_SOURCE_EXTS,
    upstream_providers = [
        [_rust_common.crate_info],
        [_rust_common.test_crate_info],
    ],
    doc = "Forwards upstream Rust providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Rust sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream rust_* target whose providers are preserved.",
)

# Advertised providers for the Cc-linking shapes (`rust_shared_library`,
# `rust_static_library`). Mirrors `_DX_FORWARD_PROVIDES` with
# `TestCrateInfo` in place of `CrateInfo` (upstream provides no `CrateInfo`
# for these shapes) plus the advertised `CcInfo` linking surface. The
# advertised `TestCrateInfo` is load-bearing beyond preservation: the
# upstream lint aspects match `required_providers` against advertised
# providers, so without it `rustfmt_test`/`rust_clippy_test` over the
# wrappers would pass vacuously.
_DX_CC_FORWARD_PROVIDES = [
    _rust_common.test_crate_info,
    _rust_common.dep_info,
    CcInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_rust_forward_cc = dx_library_forward_rule(
    provides = _DX_CC_FORWARD_PROVIDES,
    required_providers = [(_rust_common.test_crate_info, "TestCrateInfo"), (_rust_common.dep_info, "DepInfo"), (CcInfo, "CcInfo")],
    quality_specs = _DX_RUST_SOURCE_SPECS,
    what = "rust_*",
    allow_files = _DX_RUST_SOURCE_EXTS,
    upstream_providers = [[_rust_common.test_crate_info]],
    doc = "Forwards the upstream Cc-linking providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Rust sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream rust_shared_library/rust_static_library target whose providers are preserved.",
)

_rust_forward_binary = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_FORWARD_PROVIDES,
    required_providers = _DX_RUST_CRATE_PROVIDERS,
    quality_specs = _DX_RUST_SOURCE_SPECS,
    what = "rust_*",
    allow_files = _DX_RUST_SOURCE_EXTS,
    upstream_providers = [
        [_rust_common.crate_info],
        [_rust_common.test_crate_info],
    ],
    doc = "Executable forwarder for rust_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Rust sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream rust_binary target whose providers are preserved.",
)

_rust_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_FORWARD_PROVIDES,
    required_providers = _DX_RUST_CRATE_PROVIDERS,
    quality_specs = _DX_RUST_SOURCE_SPECS,
    what = "rust_*",
    allow_files = _DX_RUST_SOURCE_EXTS,
    upstream_providers = [
        [_rust_common.crate_info],
        [_rust_common.test_crate_info],
    ],
    doc = "Test forwarder for rust_test: symlinks the upstream test executable.",
    srcs_doc = "Direct Rust sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream rust_test target whose providers are preserved.",
    extra_attrs = dx_lcov_merger_attr(),
)

def rust_library(
        name,
        srcs,
        crate_name = None,
        edition = RUST_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_library` (M02)."""
    dx_wrap(
        name,
        _rust_library,
        _rust_forward,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def rust_binary(
        name,
        srcs,
        crate_name = None,
        edition = RUST_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_binary` (M02)."""
    dx_wrap(
        name,
        _rust_binary,
        _rust_forward_binary,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def rust_test(
        name,
        srcs = None,
        crate = None,
        edition = RUST_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_test` (M02).

    With `crate`, the referenced wrapper stays the single source owner and
    this target reports no direct sources. With `srcs`, those sources are
    this test's direct sources.

    Args:
      name: public test target name (upstream target is name_upstream).
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
        name = name + "_upstream",
        **upstream_kwargs
    )
    _rust_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
    )

def rust_proc_macro(
        name,
        srcs,
        crate_name = None,
        edition = RUST_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_proc_macro` (M12).

    Same forwarding shape as `rust_library`: the private upstream keeps
    the crate providers and the public target adds QualitySourcesInfo.
    """
    dx_wrap(
        name,
        _rust_proc_macro,
        _rust_forward,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def rust_shared_library(
        name,
        srcs,
        crate_name = None,
        edition = RUST_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_shared_library` (M12).

    Cc-linking forwarding shape: the private upstream keeps the `CcInfo`
    linking context (plus the `TestCrateInfo`-wrapped crate for `rust_test`)
    and the public target adds QualitySourcesInfo. Upstream provides no
    `CrateInfo` for this shape, so unlike `rust_library` there is none
    to preserve.
    """
    dx_wrap(
        name,
        _rust_shared_library,
        _rust_forward_cc,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def rust_static_library(
        name,
        srcs,
        crate_name = None,
        edition = RUST_EDITION,
        visibility = None,
        **kwargs):
    """Experimental minimal wrapper over `rust_static_library` (M12).

    Cc-linking forwarding shape, mirroring `rust_shared_library`.
    """
    dx_wrap(
        name,
        _rust_static_library,
        _rust_forward_cc,
        srcs,
        crate_name = crate_name,
        edition = edition,
        visibility = visibility,
        **kwargs
    )

def dx_rust_crate(
        name,
        package_name,
        deps,
        dev_deps = None,
        extra_deps = None,
        extra_test_deps = None,
        crate_name = None,
        size = "small",
        visibility = None):
    """Single-crate boilerplate: lib + test + lint tests + manifest + corpus (issue #239).

    Emits the M02 leaf-crate pattern with names identical to the
    hand-written stanzas it replaces, so migration is a pure BUILD-text
    change: `<name>` (`rust_library` over `src/lib.rs`), `<name>_test`
    (`rust_test` via `crate`), `<name>_fmt_test` / `<name>_clippy_test`
    over the library, `exports_files(["Cargo.toml"])` (always public:
    crate_universe reads the manifest from the `@crates` repo), and the
    `corpus` `real_source_target` owning `BUILD.bazel` + `Cargo.toml`
    for direct-Bazel dogfood. Dependency labels resolve through
    crate_universe exactly like the hand-written calls: `deps` /
    `dev_deps` are crate names, `extra_deps` / `extra_test_deps` are
    literal labels appended after the resolved ones.

    Crates with binaries keep hand-written `rust_binary` stanzas (and
    lint `targets` covering them): see e.g. `quality/evaluator`, whose
    `quality_evaluator` binary shares the crate name with the lib.

    Args:
      name: library target and crate stem (test is `<name>_test`, lint
        tests `<name>_fmt_test` / `<name>_clippy_test`).
      package_name: crate_universe package (e.g. `cli/dx_fingerprint`).
      deps: normal crate names for lib and test.
      dev_deps: test-only crate names.
      extra_deps: literal labels appended to lib and test deps.
      extra_test_deps: literal labels appended to test deps only.
      crate_name: Rust crate name, defaults to `name`.
      size: test size for the unit and lint tests.
      visibility: visibility of the library and test forwarders.
    """
    crate = name if crate_name == None else crate_name
    lib_deps = _crate_deps(
        deps,
        package_name = package_name,
    ) + (extra_deps or [])
    rust_library(
        name = name,
        srcs = ["src/lib.rs"],
        aliases = _aliases(
            package_name = package_name,
            normal = True,
        ),
        crate_name = crate,
        crate_root = "src/lib.rs",
        deps = lib_deps,
        visibility = visibility,
    )
    rust_test(
        name = name + "_test",
        size = size,
        aliases = _aliases(
            package_name = package_name,
            normal = True,
            normal_dev = True,
        ),
        crate = ":" + name,
        deps = _crate_deps(
            deps + (dev_deps or []),
            package_name = package_name,
        ) + (extra_deps or []) + (extra_test_deps or []),
        visibility = visibility,
    )
    _rustfmt_test(
        name = name + "_fmt_test",
        size = size,
        targets = [":" + name],
    )
    _rust_clippy_test(
        name = name + "_clippy_test",
        size = size,
        targets = [":" + name],
    )
    native.exports_files(
        ["Cargo.toml"],
        visibility = ["//visibility:public"],
    )
    real_source_target(
        name = "corpus",
        starlark_srcs = ["BUILD.bazel"],
        toml_srcs = ["Cargo.toml"],
    )
