"""Experimental minimal PowerShell wrappers (ADR 0032).

Upstream: rules_powershell 0.2.0 plus portable pwsh 7.5.4 (MODULE.bazel).
"""

load("@rules_powershell//powershell:defs.bzl", _PwshInfo = "PwshInfo", _pwsh_binary = "pwsh_binary", _pwsh_library = "pwsh_library", _pwsh_test = "pwsh_test")
load("//libs/starlark:wrapper.bzl", "dx_executable_forward_rule", "dx_lcov_merger_attr", "dx_library_forward_rule", "dx_wrap", "dx_wrap_binary", "dx_wrap_test")
load("//quality:sources.bzl", "QualitySourcesInfo")

_DX_POWERSHELL_LIBRARY_PROVIDES = [
    _PwshInfo,
    DefaultInfo,
    InstrumentedFilesInfo,
    QualitySourcesInfo,
]

_DX_POWERSHELL_EXEC_PROVIDES = [
    DefaultInfo,
    QualitySourcesInfo,
]

_DX_POWERSHELL_LIBRARY_SPECS = [("powershell", ["ps1", "psm1", "psd1"])]
_DX_POWERSHELL_LIBRARY_EXTS = [".ps1", ".psm1", ".psd1"]
_DX_POWERSHELL_EXEC_SPECS = [("powershell", ["ps1"])]
_DX_POWERSHELL_EXEC_EXTS = [".ps1"]

_pwsh_library_forward = dx_library_forward_rule(
    provides = _DX_POWERSHELL_LIBRARY_PROVIDES,
    required_providers = [(_PwshInfo, "PwshInfo")],
    quality_specs = _DX_POWERSHELL_LIBRARY_SPECS,
    what = "pwsh_*",
    allow_files = _DX_POWERSHELL_LIBRARY_EXTS,
    upstream_providers = [[_PwshInfo]],
    doc = "Forwards upstream PowerShell library providers unchanged and adds QualitySourcesInfo.",
    srcs_doc = "Direct PowerShell sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream pwsh_library target whose providers are preserved.",
    runtime = "besteffort",
)

_pwsh_binary_forward = dx_executable_forward_rule(
    kind = "executable",
    provides = _DX_POWERSHELL_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_POWERSHELL_EXEC_SPECS,
    what = "pwsh_*",
    allow_files = _DX_POWERSHELL_EXEC_EXTS,
    upstream_providers = [[_PwshInfo]],
    doc = "Executable forwarder for pwsh_binary: symlinks the upstream binary.",
    srcs_doc = "Direct PowerShell sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream pwsh_binary target whose executable is symlinked.",
    optional_providers = [_PwshInfo],
    runtime = "besteffort",
)

_pwsh_forward_test = dx_executable_forward_rule(
    kind = "test",
    provides = _DX_POWERSHELL_EXEC_PROVIDES,
    required_providers = [],
    quality_specs = _DX_POWERSHELL_EXEC_SPECS,
    what = "pwsh_*",
    allow_files = _DX_POWERSHELL_EXEC_EXTS,
    upstream_providers = [[_PwshInfo]],
    doc = "Test forwarder for pwsh_test: symlinks the upstream test executable.",
    srcs_doc = "Direct PowerShell test sources owned by this wrapper for QualitySourcesInfo.",
    upstream_doc = "The private upstream pwsh_test target whose executable is symlinked.",
    extra_attrs = dx_lcov_merger_attr(),
    optional_providers = [_PwshInfo],
)

def powershell_effective_srcs(srcs):
    """Returns the effective direct sources for a PowerShell binary or test shape.

    None becomes empty.
    """
    return srcs if srcs != None else []

def _pwsh_wrap_library(name, srcs, visibility = None, **kwargs):
    dx_wrap(name, _pwsh_library, _pwsh_library_forward, srcs, visibility = visibility, **kwargs)

def _pwsh_wrap_binary(name, srcs, visibility = None, **kwargs):
    dx_wrap_binary(name, _pwsh_binary, _pwsh_binary_forward, srcs, visibility = visibility, **kwargs)

def pwsh_library(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `pwsh_library`.

    Aggregates `.ps1`/`.psm1`/`.psd1` sources plus `deps` on wrapper
    libraries (module `imports` for `PSModulePath` setup). At most one
    `.psd1` manifest per library is enforced upstream."""
    _pwsh_wrap_library(name, srcs, visibility = visibility, **kwargs)

def pwsh_binary(name, srcs = None, visibility = None, **kwargs):
    """Experimental minimal wrapper over `pwsh_binary`.

    An ordinary binary owns exactly one `.ps1` entry `srcs` plus `deps`
    on wrapper libraries for module imports. A thin entry shape with no
    `srcs` reports no direct sources and fails upstream fail-closed
    (upstream requires exactly one entry file). Both shapes preserve the
    upstream providers and execution semantics over the portable `pwsh`
    runtime."""
    effective_srcs = powershell_effective_srcs(srcs)
    _pwsh_wrap_binary(name, effective_srcs, visibility = visibility, **kwargs)

def pwsh_test(name, srcs, visibility = None, **kwargs):
    """Experimental minimal wrapper over `pwsh_test`.

    With `srcs`, those test sources are this test's direct sources for
    QualitySourcesInfo. The library under test stays its ordinary owner
    via `deps`; tested sources are never this test's direct sources.
    Plain-executable tests (exit code is the verdict) stay supported; the
    Pester 5.7.1 mapping is qualified (`powershell/tests/fixtures/pester/`:
    `Describe`/`It` sources plus `Invoke-Pester` entry over the pinned
    Gallery lock; unpinned runner rejected). Uses Bazel's standard test
    and coverage     protocols over the portable `pwsh` runtime."""
    dx_wrap_test(name, _pwsh_test, _pwsh_forward_test, srcs, visibility = visibility, **kwargs)
