"""Experimental minimal Ruby wrappers (ADR 0032).

Contract: `docs/decisions/0032-ruby-powershell-bandit-swift.md`.
Upstream: rules_ruby 0.28.0 plus Ruby 3.4.9 (MODULE.bazel).
"""

load("@rules_ruby//ruby:defs.bzl", _rb_binary = "rb_binary", _rb_library = "rb_library", _rb_test = "rb_test")
load("@rules_ruby//ruby/private:providers.bzl", "BundlerInfo", "RubyFilesInfo")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap", "dx_wrap_binary", "dx_wrap_test")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_RUBY_LIBRARY_PROVIDES = [
    RubyFilesInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

# NB: binaries and tests forward the upstream `RubyFilesInfo` plus
_DX_RUBY_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

# Ruby owns `.rb` only. `Gemfile`/`Gemfile.lock` stay fixture-owned
# no manifest `srcs`, so Bundler manifests run via depcheck fixtures,
# never these wrappers.
_DX_RUBY_SOURCE_SPECS = [("ruby", "rb")]
_DX_RUBY_SOURCE_EXTS = [".rb"]

_ruby_library_forward = dx_library_forward_rule(
    provides = _DX_RUBY_LIBRARY_PROVIDES,
    required_providers = [(RubyFilesInfo, "RubyFilesInfo")],
    quality_specs = _DX_RUBY_SOURCE_SPECS,
    what = "ruby_*",
    allow_files = _DX_RUBY_SOURCE_EXTS,
    upstream_providers = [[RubyFilesInfo]],
    doc = "Forwards upstream Ruby library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct Ruby sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream rb_library target whose providers are preserved.",
    runtime = "besteffort",
)

_ruby_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_RUBY_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_RUBY_SOURCE_SPECS,
    what = "ruby_*",
    allow_files = _DX_RUBY_SOURCE_EXTS,
    upstream_providers = [[RubyFilesInfo]],
    doc = "Executable forwarder for ruby_binary: symlinks the upstream binary.",
    srcs_doc = "Direct Ruby sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream rb_binary target whose executable is symlinked.",
    optional_providers = [RubyFilesInfo, BundlerInfo],
    runtime = "besteffort",
)

_ruby_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_RUBY_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_RUBY_SOURCE_SPECS,
    what = "ruby_*",
    allow_files = _DX_RUBY_SOURCE_EXTS,
    upstream_providers = [[RubyFilesInfo]],
    doc = "Test forwarder for ruby_test: symlinks the upstream test executable.",
    srcs_doc = "Direct Ruby test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream rb_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [RubyFilesInfo, BundlerInfo],
    runtime = "besteffort",
)

def ruby_effective_srcs(srcs):
    """Returns the effective direct sources for a ruby binary or test shape.

    None becomes empty (thin entry shape owns no direct sources).
    See: docs/quality/quality-sources.md."""
    return srcs if srcs != None else []

def _ruby_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _rb_library, _ruby_library_forward, srcs, visibility = visibility, **kwargs)

def _ruby_wrap_binary(name, srcs, visibility = None, **kwargs):
    dx_wrap_binary(name, _rb_binary, _ruby_binary_forward, srcs, visibility = visibility, **kwargs)

def ruby_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `rb_library`."""
    _ruby_wrap_library(name, srcs, visibility = visibility, **kwargs)

def ruby_binary(name, srcs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `rb_binary`.

    Two shapes: an ordinary binary owns its `srcs` plus `deps` on a wrapper
    library, while a thin entry binary generated for a recognized entry
    carries only `main` plus `deps = [":<library>"]` with no `srcs`. The
    library alone owns the source and its source-derived dependencies in
    the thin shape; the thin binary reports no direct sources. Both shapes
    preserve the upstream providers and execution semantics.
    """
    effective_srcs = ruby_effective_srcs(srcs)
    if "srcs" in kwargs:
        kwargs.pop("srcs")
    _ruby_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def ruby_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `rb_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner via
    `deps`; test sources are never the library's sources. RSpec runs through
    `main = "@bundle//bin:rspec"` with `args` plus `deps` on the spec helper
    and `@bundle` (no new rule kind); plain `rb_test` executables stay
    supported. Uses Bazel's standard test and coverage protocols.
    """
    dx_wrap_test(name, _rb_test, _ruby_forward_test, srcs, visibility = visibility, **kwargs)
