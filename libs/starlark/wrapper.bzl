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
  them.
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
      specs: list of `(class ID, file extension without dot)` tuples in
        deterministic order; only non-empty classes are recorded.
      label: subject label rendered in validation diagnostics.

    Returns:
      The `QualitySourcesInfo` provider for the direct sources.
    """
    direct_sources = {}
    for spec in specs:
        matched = [f for f in files if f.extension == spec[1]]
        if len(matched) > 0:
            direct_sources[spec[0]] = depset(matched)
    check_direct_sources(direct_sources, label)
    return QualitySourcesInfo(direct_sources = direct_sources)

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

def dx_forward_attrs(allow_files, srcs_doc, upstream_providers, upstream_doc, extra_attrs = None):
    """Builds the common `srcs`/`upstream` attribute dict for forwarders.

    Args:
      allow_files: file extensions the wrapper owns for `QualitySourcesInfo`.
      srcs_doc: documentation for the `srcs` attribute.
      upstream_providers: provider-list constraint for the `upstream`
        attribute (for example `[[_JsInfo]]`).
      upstream_doc: documentation for the `upstream` attribute.
      extra_attrs: optional additional attributes (for example the
        coverage `_lcov_merger` on test forwarders).

    Returns:
      The attribute dict for the forwarding `rule()`.
    """
    attrs = {
        "srcs": attr.label_list(
            allow_files = allow_files,
            doc = srcs_doc,
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = upstream_providers,
            doc = upstream_doc,
        ),
    }
    if extra_attrs:
        attrs.update(extra_attrs)
    return attrs

def dx_library_forward_rule(provides, required_providers, quality_specs, what, allow_files, upstream_providers, doc, srcs_doc, upstream_doc):
    """Creates the public forwarding rule for one library wrapper.

    The implementation preserves the required upstream providers plus
    `DefaultInfo`, forwards the shared runtime providers, and adds
    `QualitySourcesInfo` for the direct sources.

    Args:
      provides: provider list advertised by the forwarding rule.
      required_providers: list of `(provider, display name)` tuples the
        upstream target must provide.
      quality_specs: list of `(class ID, file extension)` tuples for
        direct-source normalization.
      what: wrapper family prefix for diagnostics.
      allow_files: file extensions the wrapper owns.
      upstream_providers: provider-list constraint for the `upstream`
        attribute.
      doc: documentation for the forwarding rule.
      srcs_doc: documentation for the `srcs` attribute.
      upstream_doc: documentation for the `upstream` attribute.

    Returns:
      The forwarding rule callable.
    """

    def _impl(ctx):
        upstream = ctx.attr.upstream
        return (
            dx_preserved_providers(upstream, required_providers, what) +
            [upstream[DefaultInfo]] +
            dx_forwarded_runtime_providers(upstream, what) +
            [dx_quality_sources(ctx.files.srcs, quality_specs, str(ctx.label))]
        )

    return rule(
        implementation = _impl,
        provides = provides,
        attrs = dx_forward_attrs(
            allow_files = allow_files,
            srcs_doc = srcs_doc,
            upstream_providers = upstream_providers,
            upstream_doc = upstream_doc,
        ),
        doc = doc,
    )

def dx_executable_forward_rule(kind, provides, required_providers, quality_specs, what, allow_files, upstream_providers, doc, srcs_doc, upstream_doc, extra_attrs = None):
    """Creates the executable or test forwarding rule for one wrapper.

    The implementation symlinks the upstream executable into its own
    declared output, then preserves the required upstream providers,
    forwards the shared runtime providers, and adds `QualitySourcesInfo`
    for the direct sources.

    Args:
      kind: `"executable"` for binaries, `"test"` for tests.
      provides: provider list advertised by the forwarding rule.
      required_providers: list of `(provider, display name)` tuples the
        upstream target must provide.
      quality_specs: list of `(class ID, file extension)` tuples for
        direct-source normalization.
      what: wrapper family prefix for diagnostics.
      allow_files: file extensions the wrapper owns.
      upstream_providers: provider-list constraint for the `upstream`
        attribute.
      doc: documentation for the forwarding rule.
      srcs_doc: documentation for the `srcs` attribute.
      upstream_doc: documentation for the `upstream` attribute.
      extra_attrs: optional additional attributes (for example the
        coverage `_lcov_merger` on test forwarders).

    Returns:
      The forwarding rule callable.
    """

    def _impl(ctx):
        upstream = ctx.attr.upstream
        return (
            [dx_symlink_default_info(ctx, what)] +
            dx_preserved_providers(upstream, required_providers, what) +
            dx_forwarded_runtime_providers(upstream, what) +
            [dx_quality_sources(ctx.files.srcs, quality_specs, str(ctx.label))]
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

    Args:
      name: public wrapper target name (upstream target is `name_upstream`).
      upstream_rule: the upstream rule to instantiate privately.
      forward_rule: the public forwarding rule sharing this wrapper's `srcs`.
      srcs: direct sources owned by the wrapper.
      visibility: visibility of the public forwarding target.
      **kwargs: extra attributes forwarded to the upstream rule.
    """
    upstream_rule(
        name = name + "_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    forward_rule(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
    )
