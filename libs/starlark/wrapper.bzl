"""Shared wrapper-forwarder plumbing for language rules (issue #97).

Every language `rules/defs.bzl` repeats the same shape: one private
`<name>_upstream` target plus one public forwarding rule that preserves
the upstream providers unchanged and adds `QualitySourcesInfo`
normalized from the wrapper's direct `srcs`. This module owns that
plumbing once, parameterized by docs and provider names, so wrapper
fixes and `QualitySourcesInfo` changes land in one place.

Forwarding semantics (unchanged from the per-language copies):

- Preserved providers must all be present on the upstream target;
  otherwise analysis fails naming the missing provider.
- `InstrumentedFilesInfo` is always forwarded; `OutputGroupInfo` and
  `RunEnvironmentInfo` are forwarded when the upstream target provides
  them. Some families (Go/CC/Java binaries, .NET shapes) forward every
  runtime provider best-effort instead; those pass `runtime = "besteffort"`.
- `QualitySourcesInfo` covers direct sources only, keyed by semantic
  class ID; transitive sources stay readable from the preserved
  providers.
- Executable and test forwarders symlink the upstream executable into
  their own declared output (Bazel requires executable-providing rules
  to create the file themselves).
- Macro changes here affect forwarding plumbing only, never
  toolchain or registry semantics (per #7).
"""

load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

def dx_forwarded_runtime_providers(upstream, what):
    """Forwards the upstream runtime providers every wrapper preserves.

    `InstrumentedFilesInfo` must be present; `OutputGroupInfo` and
    `RunEnvironmentInfo` are forwarded when available.

    Args:
      upstream: the private upstream target whose providers are preserved.
      what: wrapper family prefix for diagnostics (for example `"mdx_*"`).

    Returns:
      The list of preserved runtime provider instances.
    """
    out = []
    if InstrumentedFilesInfo not in upstream:
        fail(what + ": upstream target has no InstrumentedFilesInfo: " + str(upstream.label))
    out.append(upstream[InstrumentedFilesInfo])
    if OutputGroupInfo in upstream:
        out.append(upstream[OutputGroupInfo])
    if RunEnvironmentInfo in upstream:
        out.append(upstream[RunEnvironmentInfo])
    return out

def dx_forwarded_optional(upstream, providers):
    """Forwards the upstream providers that are present (best-effort).

    Unlike `dx_forwarded_runtime_providers`, a missing provider is
    silently skipped instead of failing. Used for the executable shapes
    that mirror their upstream target's optional surfaces (Go archives,
    CcInfo/JavaInfo on binaries, .NET assemblies, coverage metadata on
    shapes whose upstream may omit it).

    Args:
      upstream: the private upstream target whose providers are preserved.
      providers: provider objects to forward when present, in order.

    Returns:
      The present provider instances in `providers` order.
    """
    return [upstream[p] for p in providers if p in upstream]

def dx_preserved_providers(upstream, required, what):
    """Returns the required upstream provider instances, failing when absent.

    Args:
      upstream: the private upstream target whose providers are preserved.
      required: list of `(provider, display name)` tuples in forward order.
      what: wrapper family prefix for diagnostics (for example `"python_*"`).

    Returns:
      The required provider instances in `required` order.
    """
    out = []
    for item in required:
        provider = item[0]
        name = item[1]
        if provider not in upstream:
            fail(what + ": upstream target has no " + name + ": " + str(upstream.label))
        out.append(upstream[provider])
    return out

def dx_quality_sources(files, specs, label):
    """Builds `QualitySourcesInfo` for direct wrapper sources.

    Args:
      files: direct source files (usually `ctx.files.srcs`).
      specs: list of `(class ID, extension)` tuples in deterministic
        order; only non-empty classes are recorded. The extension may be
        a single extension without dot or a list of them, and a spec may
        carry an optional third element with basename suffixes to exclude
        (for example `("typescript", ["ts", "mts", "cts"],
        [".d.ts", ".d.mts", ".d.cts"])`). Specs sharing one class ID
        merge into a single depset.
      label: subject label rendered in validation diagnostics.

    Returns:
      The `QualitySourcesInfo` provider for the direct sources.
    """
    buckets = {}
    order = []
    for spec in specs:
        key = spec[0]
        exts = spec[1] if type(spec[1]) == "list" else [spec[1]]
        excludes = spec[2] if len(spec) > 2 else []
        matched = [
            f
            for f in files
            if f.extension in exts and not _has_excluded_suffix(f.basename, excludes)
        ]
        if key not in buckets:
            buckets[key] = []
            order.append(key)
        buckets[key].extend(matched)
    direct_sources = {}
    for key in order:
        if len(buckets[key]) > 0:
            direct_sources[key] = depset(buckets[key])
    check_direct_sources(direct_sources, label)
    return QualitySourcesInfo(direct_sources = direct_sources)

def _has_excluded_suffix(basename, excludes):
    for suffix in excludes:
        if basename.endswith(suffix):
            return True
    return False

def dx_symlink_default_info(ctx, what):
    """Builds the executable `DefaultInfo` symlinking the upstream binary.

    Args:
      ctx: rule context with a mandatory `upstream` attribute.
      what: wrapper family prefix for diagnostics.

    Returns:
      The `DefaultInfo` provider with the declared executable symlink.
    """
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail(what + ": upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(output = link, target_file = exe)
    return DefaultInfo(
        executable = link,
        files = depset([link]),
        runfiles = ctx.runfiles(files = [link]).merge(upstream.default_runfiles),
    )

def dx_lcov_merger_attr():
    """Returns the coverage `_lcov_merger` attribute for test forwarders.

    Bazel's coverage runner passes this magic attribute as LCOV_MERGER,
    which merges the per-test staging report into coverage.dat; without
    it the runner exits after touching an empty file even though the
    test collected coverage. Same declaration as the upstream-wrapping
    test forwarders.

    Returns:
      The `{"_lcov_merger": ...}` attribute dict for `dx_forward_attrs`.
    """
    return {
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
    }

def dx_forward_attrs(allow_files, srcs_doc, upstream_providers, upstream_doc, extra_attrs = None):
    """Builds the common `srcs`/`upstream` attribute dict for forwarders.

    Args:
      allow_files: file extensions the wrapper owns for `QualitySourcesInfo`.
      srcs_doc: documentation for the `srcs` attribute.
      upstream_providers: provider-list constraint for the `upstream`
        attribute (for example `[[_JsInfo]]`), or `None` for no
        constraint.
      upstream_doc: documentation for the `upstream` attribute.
      extra_attrs: optional additional attributes (for example the
        coverage `_lcov_merger` on test forwarders, or `hdrs` on the C++
        library forwarder).

    Returns:
      The attribute dict for the forwarding `rule()`.
    """
    upstream_attr_kwargs = {
        "doc": upstream_doc,
        "mandatory": True,
    }
    if upstream_providers != None:
        upstream_attr_kwargs["providers"] = upstream_providers
    attrs = {
        "srcs": attr.label_list(
            allow_files = allow_files,
            doc = srcs_doc,
        ),
        "upstream": attr.label(**upstream_attr_kwargs),
    }
    if extra_attrs:
        attrs.update(extra_attrs)
    return attrs

def _dx_quality_files(ctx, extra_quality_attrs):
    files = list(ctx.files.srcs)
    for name in extra_quality_attrs or []:
        files.extend(getattr(ctx.files, name, []))
    return files

def _dx_runtime_providers(upstream, what, runtime):
    if runtime == "mandatory":
        return dx_forwarded_runtime_providers(upstream, what)
    elif runtime == "besteffort":
        return dx_forwarded_optional(upstream, [InstrumentedFilesInfo, OutputGroupInfo, RunEnvironmentInfo])
    else:
        fail("dx wrapper: unknown runtime '" + runtime + "': want \"mandatory\" or \"besteffort\"")

def dx_library_forward_rule(provides, required_providers, quality_specs, what, allow_files, upstream_providers, doc, srcs_doc, upstream_doc, extra_attrs = None, runtime = "mandatory", extra_quality_attrs = None):
    """Creates the public forwarding rule for one library wrapper.

    The implementation preserves the required upstream providers plus
    `DefaultInfo`, forwards the shared runtime providers, and adds
    `QualitySourcesInfo` for the direct sources.

    Args:
      provides: provider list advertised by the forwarding rule.
      required_providers: list of `(provider, display name)` tuples the
        upstream target must provide.
      quality_specs: list of `(class ID, extension)` tuples for
        direct-source normalization (see `dx_quality_sources`).
      what: wrapper family prefix for diagnostics.
      allow_files: file extensions the wrapper owns.
      upstream_providers: provider-list constraint for the `upstream`
        attribute, or `None` for no constraint.
      doc: documentation for the forwarding rule.
      srcs_doc: documentation for the `srcs` attribute.
      upstream_doc: documentation for the `upstream` attribute.
      extra_attrs: optional additional attributes (for example `hdrs` on
        the C++ library forwarder).
      runtime: `"mandatory"` fails when `InstrumentedFilesInfo` is
        absent; `"besteffort"` forwards every runtime provider only when
        present (the .NET library shape).
      extra_quality_attrs: optional additional file attributes (for
        example `"hdrs"`) whose files join `srcs` for
        `QualitySourcesInfo`.

    Returns:
      The forwarding rule callable.
    """

    def _impl(ctx):
        upstream = ctx.attr.upstream
        return (
            dx_preserved_providers(upstream, required_providers, what) +
            [upstream[DefaultInfo]] +
            _dx_runtime_providers(upstream, what, runtime) +
            [dx_quality_sources(_dx_quality_files(ctx, extra_quality_attrs), quality_specs, str(ctx.label))]
        )

    return rule(
        implementation = _impl,
        provides = provides,
        attrs = dx_forward_attrs(
            allow_files = allow_files,
            srcs_doc = srcs_doc,
            upstream_providers = upstream_providers,
            upstream_doc = upstream_doc,
            extra_attrs = extra_attrs,
        ),
        doc = doc,
    )

def dx_executable_forward_rule(kind, provides, required_providers, quality_specs, what, allow_files, upstream_providers, doc, srcs_doc, upstream_doc, extra_attrs = None, optional_providers = [], runtime = "mandatory", extra_quality_attrs = None):
    """Creates the executable or test forwarding rule for one wrapper.

    The implementation symlinks the upstream executable into its own
    declared output, then preserves the required upstream providers,
    forwards the best-effort optional providers plus the shared runtime
    providers, and adds `QualitySourcesInfo` for the direct sources.

    Args:
      kind: `"executable"` for binaries, `"test"` for tests.
      provides: provider list advertised by the forwarding rule.
      required_providers: list of `(provider, display name)` tuples the
        upstream target must provide.
      quality_specs: list of `(class ID, extension)` tuples for
        direct-source normalization (see `dx_quality_sources`).
      what: wrapper family prefix for diagnostics.
      allow_files: file extensions the wrapper owns.
      upstream_providers: provider-list constraint for the `upstream`
        attribute, or `None` for no constraint.
      doc: documentation for the forwarding rule.
      srcs_doc: documentation for the `srcs` attribute.
      upstream_doc: documentation for the `upstream` attribute.
      extra_attrs: optional additional attributes (for example the
        coverage `_lcov_merger` on test forwarders).
      optional_providers: provider objects forwarded best-effort when
        present (for example the language info on binaries that mirror
        their upstream target's optional surfaces).
      runtime: `"mandatory"` fails when `InstrumentedFilesInfo` is
        absent; `"besteffort"` forwards every runtime provider only when
        present (Go/CC/Java/.NET binaries and .NET tests).
      extra_quality_attrs: optional additional file attributes whose
        files join `srcs` for `QualitySourcesInfo`.

    Returns:
      The forwarding rule callable.
    """

    def _impl(ctx):
        upstream = ctx.attr.upstream
        return (
            [dx_symlink_default_info(ctx, what)] +
            dx_preserved_providers(upstream, required_providers, what) +
            dx_forwarded_optional(upstream, optional_providers) +
            _dx_runtime_providers(upstream, what, runtime) +
            [dx_quality_sources(_dx_quality_files(ctx, extra_quality_attrs), quality_specs, str(ctx.label))]
        )

    attrs = dx_forward_attrs(
        allow_files = allow_files,
        srcs_doc = srcs_doc,
        upstream_providers = upstream_providers,
        upstream_doc = upstream_doc,
        extra_attrs = extra_attrs,
    )
    if kind == "executable":
        return rule(
            implementation = _impl,
            executable = True,
            provides = provides,
            attrs = attrs,
            doc = doc,
        )
    elif kind == "test":
        return rule(
            implementation = _impl,
            test = True,
            provides = provides,
            attrs = attrs,
            doc = doc,
        )
    else:
        fail("dx_executable_forward_rule: unknown kind '" + kind + "': want \"executable\" or \"test\"")

def dx_wrap(name, upstream_rule, forward_rule, srcs, visibility = None, **kwargs):
    """Instantiates one private upstream target plus its public forwarder.

    `aspect_hints` (typed native-config labels) ride the public forwarder
    where quality aspects visit (issue #12, lane A): the forwarder is the
    `QualitySourcesInfo` owner, so hints must reach it, not only the
    private upstream. Remaining kwargs stay upstream-only.

    Args:
      name: public wrapper target name (upstream target is `name_upstream`).
      upstream_rule: the upstream rule to instantiate privately.
      forward_rule: the public forwarding rule sharing this wrapper's `srcs`.
      srcs: direct sources owned by the wrapper.
      visibility: visibility of the public forwarding target.
      **kwargs: extra attributes forwarded to the upstream rule (`aspect_hints`
        additionally forwards to the public target).
    """
    hints = kwargs.get("aspect_hints", None)
    upstream_rule(
        name = name + "_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    forward_kwargs = {}
    if hints != None:
        forward_kwargs["aspect_hints"] = hints
    forward_rule(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
        **forward_kwargs
    )
