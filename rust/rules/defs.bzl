"""Experimental minimal Rust wrappers (M02, ADR 0013).

Contract: `docs/decisions/0013-rust-javascript-typescript-foundations.md`, `docs/decisions/0012-language-toolchain-versions.md`.
"""

load("@crates//:crates.bzl", _aliases = "aliases", _crate_deps = "crate_deps")
load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load("@rules_rust//rust:defs.bzl", _rust_binary = "rust_binary", _rust_clippy_test = "rust_clippy_test", _rust_common = "rust_common", _rust_library = "rust_library", _rust_proc_macro = "rust_proc_macro", _rust_shared_library = "rust_shared_library", _rust_static_library = "rust_static_library", _rust_test = "rust_test", _rustfmt_test = "rustfmt_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap")
load("//quality:sources.bzl", "QualitySourcesInfo", "RUST")

# RUST_EDITION is the single source of truth (issue #82), defined in
# `:edition.bzl` so consumer aspects stay crate-free (issue #55).
load(":edition.bzl", "RUST_EDITION")

# Advertised providers: Bazel matches aspects on `provides`, not returned
# providers, so forwarders must advertise or lint aspects skip vacuously.
# See `libs/starlark/wrapper.bzl`.
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

# Cc-linking shapes use `TestCrateInfo` (no `CrateInfo` upstream) plus `CcInfo`.
# See `libs/starlark/wrapper.bzl`.
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
    this test's direct sources."""
    test_srcs = srcs if srcs != None else []
    upstream_kwargs = dict(kwargs)
    upstream_kwargs["crate"] = crate
    upstream_kwargs["edition"] = edition
    upstream_kwargs.setdefault("crate_name", name)

    # The private upstream test stays an implementation detail via private
    # visibility; both it and the public wrapper run under `bazel test //...`
    # (issue #406: no manual; double-execution is the cost of green suites).
    if "tags" in upstream_kwargs:
        kept = [t for t in upstream_kwargs["tags"] if t != "manual"]
        if len(kept) > 0:
            upstream_kwargs["tags"] = kept
        else:
            upstream_kwargs.pop("tags")
    upstream_kwargs["visibility"] = ["//visibility:private"]
    if srcs != None:
        upstream_kwargs["srcs"] = srcs
    _rust_test(
        name = name + "_upstream",
        **upstream_kwargs
    )
    forward_kwargs = {}
    if "aspect_hints" in kwargs:
        # Lane A (issue #12): hints ride the QualitySourcesInfo owner
        # where aspects visit, not only the private upstream.
        forward_kwargs["aspect_hints"] = kwargs["aspect_hints"]
    _rust_forward_test(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = test_srcs,
        visibility = visibility,
        **forward_kwargs
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

def rustfmt_test(name, targets, size = "small", **kwargs):
    """Thin wrapper over upstream `rustfmt_test` (issue #239).

    Forwards unchanged to the pinned toolchain test. Consumers load this
    symbol from `//rust/rules:defs.bzl` so lint entry points stay
    single-sourced with the `rust_*` build wrappers; lint precision comes
    from the forwarder `provides` above, not from logic here.
    """
    _rustfmt_test(
        name = name,
        targets = targets,
        size = size,
        **kwargs
    )

def rust_clippy_test(name, targets, size = "small", **kwargs):
    """Thin wrapper over upstream `rust_clippy_test` (issue #239).

    Same single-source rationale as `rustfmt_test`: consumers load lints
    from this module, never from `@rules_rust` directly.
    """
    _rust_clippy_test(
        name = name,
        targets = targets,
        size = size,
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
        srcs = None,
        size = "small",
        visibility = None):
    """Single-crate boilerplate: lib + test + lint tests + manifest (issue #239).

    Emits the M02 leaf-crate pattern with names identical to the
    hand-written stanzas it replaces, so migration is a pure BUILD-text
    change: `<name>` (`rust_library` over `srcs`), `<name>_test`
    (`rust_test` via `crate`), `<name>_fmt_test` / `<name>_clippy_test`
    over the library, and `exports_files(["Cargo.toml"])` (always public:
    crate_universe reads the manifest from the `@crates` repo). Corpus
    splits (`corpus_starlark` owning `BUILD.bazel` plus any `*.bzl` like
    `roots.bzl`, `corpus_toml` owning `Cargo.toml`) are owned by `dx
    generate` (issue #15), never by this macro, so dogfood stays
    generator-stable. Dependency labels resolve through
    crate_universe exactly like the hand-written calls: `deps` /
    `dev_deps` are crate names, `extra_deps` / `extra_test_deps` are
    literal labels appended after the resolved ones.

    Crates with binaries keep hand-written `rust_binary` stanzas (and
    lint `targets` covering them): see e.g. `quality/evaluator`, whose
    `quality_evaluator` binary shares the crate name with the lib."""
    crate = name if crate_name == None else crate_name
    lib_srcs = srcs or ["src/lib.rs"]
    lib_deps = _crate_deps(
        deps,
        package_name = package_name,
    ) + (extra_deps or [])
    rust_library(
        name = name,
        srcs = lib_srcs,
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
    rustfmt_test(
        name = name + "_fmt_test",
        size = size,
        targets = [":" + name],
    )
    rust_clippy_test(
        name = name + "_clippy_test",
        size = size,
        targets = [":" + name],
    )
    native.exports_files(
        ["Cargo.toml"],
        visibility = ["//visibility:public"],
    )
