"""Shared wrapper-forwarder plumbing for language rules.

Contract: `docs/quality/quality-sources.md`.
"""

load("//quality:sources.bzl", "QualitySourcesInfo", "check_direct_sources")

def dx_forwarded_runtime_providers(upstream, what):
    """Forwards the upstream runtime providers every wrapper preserves.

    `InstrumentedFilesInfo` must be present; `OutputGroupInfo` and
    `RunEnvironmentInfo` are forwarded when available.
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

def dx_missing_optional_names(requested_names, present_names):
    """Returns the requested names absent from the present names. See"""
    return [n for n in requested_names if n not in present_names]

def dx_optional_forward_warning(what, upstream_label, requested_names, missing_names):
    """Returns the skip warning for an optional forward, or None when nothing was skipped. See"""
    if len(missing_names) == 0:
        return None
    forwarded = len(requested_names) - len(missing_names)
    suffix = "; empty forward is expected when upstream omits the surface" if forwarded == 0 else ""
    return what + ": upstream " + upstream_label + " omits optional provider(s) " + ", ".join(missing_names) + " (forwarded " + str(forwarded) + " of " + str(len(requested_names)) + ")" + suffix

def dx_forwarded_optional(upstream, providers, what = "dx wrapper"):
    """Forwards the upstream providers that are present, warning on each skip.

    Unlike `dx_forwarded_runtime_providers`, a missing provider warns
    instead of failing. Used for the executable shapes that mirror
    their upstream target's optional surfaces (Go archives,
    CcInfo/JavaInfo on binaries, .NET assemblies, coverage metadata on
    shapes whose upstream may omit it). Empty forward is expected when
    upstream omits the surface. See issue #943.
    """
    missing = [p for p in providers if p not in upstream]
    if len(missing) > 0:
        warning = dx_optional_forward_warning(
            what,
            str(upstream.label),
            [str(p) for p in providers],
            [str(p) for p in missing],
        )
        if warning != None:
            print(warning)  # buildifier: disable=print  # intentional optional-provider skip warning, not debug
    return [upstream[p] for p in providers if p in upstream]

def dx_preserved_providers(upstream, required, what):
    """Returns the required upstream provider instances, failing when absent."""
    out = []
    for item in required:
        provider = item[0]
        name = item[1]
        if provider not in upstream:
            fail(what + ": upstream target has no " + name + ": " + str(upstream.label))
        out.append(upstream[provider])
    return out

def dx_effective_visibility(visibility):
    """Returns the explicit forwarder visibility for a public export.

    Pass the caller's `visibility` through `dx_effective_visibility` when
    the forwarder is a public export that must not inherit a public
    package default implicitly; `None` becomes private so the export is
    explicit at the call site instead of leaking via the package default.
    `dx_wrap` itself inherits the package default (passthrough) so existing
    fixtures keep their `//:__subpackages__` scope; public facades like
    `//dx` and `//deploy/rules` pass explicit lists. See issue #928.
    """
    if visibility == None:
        return ["//visibility:private"]
    return visibility

def dx_forwarded_test_kwargs(kwargs):
    """Extracts the standard test attributes a test forwarder preserves.

    `tags` (minus `manual`, so both the private upstream and the public
    wrapper run under `//...`), `timeout`, `shard_count`, and `size` ride
    the forwarder; `flaky` stays upstream-only so the public forwarder is
    an ordinary test (`//tools/ci:target_tags`). Remaining kwargs stay
    upstream-only. See issue #928.
    """
    out = {}
    if "tags" in kwargs and kwargs["tags"] != None:
        kept = [t for t in kwargs["tags"] if t != "manual"]
        if len(kept) > 0:
            out["tags"] = kept
    for key in ("timeout", "shard_count", "size"):
        if key in kwargs and kwargs[key] != None:
            out[key] = kwargs[key]
    return out

def dx_quality_sources(files, specs, label):
    """Builds `QualitySourcesInfo` for direct wrapper sources."""
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

def dx_symlink_executable_name(name, is_windows):
    """Maps one forwarder output name to its host-native filename."""
    if is_windows:
        return name + ".exe"
    return name

def dx_symlink_windows_attr():
    """Returns the `_windows_os` attribute detecting Windows target platforms."""
    return {
        "_windows_os": attr.label(
            default = "@platforms//os:windows",
            doc = "Constraint value detecting Windows target platforms for executable naming.",
        ),
    }

def dx_symlink_is_windows(ctx):
    """Returns whether the forwarder builds for a Windows target platform."""
    return ctx.target_platform_has_constraint(
        ctx.attr._windows_os[platform_common.ConstraintValueInfo],
    )

def dx_symlink_executable(ctx, target_file):
    """Symlinks one upstream executable with platform-aware naming and attrs."""
    link = ctx.actions.declare_file(
        dx_symlink_executable_name(ctx.label.name, dx_symlink_is_windows(ctx)),
    )
    ctx.actions.symlink(output = link, target_file = target_file, is_executable = True)
    return link

def dx_symlink_default_info(ctx, what):
    """Builds the executable `DefaultInfo` symlinking the upstream binary."""
    upstream = ctx.attr.upstream[DefaultInfo]
    exe = upstream.files_to_run.executable
    if exe == None:
        fail(what + ": upstream target has no executable: " + str(ctx.attr.upstream.label))
    link = dx_symlink_executable(ctx, exe)
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
    test forwarders."""
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

    `upstream` is a single same-package private label in the same
    configuration, so it takes no `cfg` (no transition) and no
    `allow_files` (a rule target, never a source file). The sealed
    `providers` list is the fail-closed contract: Bazel rejects a wrong
    upstream at analysis. See issue #928.
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

def _dx_runtime_providers(ctx, upstream, what, runtime, extra_quality_attrs = None):
    if runtime == "mandatory":
        return dx_forwarded_runtime_providers(upstream, what)
    elif runtime == "besteffort":
        out = []
        if InstrumentedFilesInfo in upstream:
            out.append(upstream[InstrumentedFilesInfo])
        else:
            # Synthesize coverage metadata from direct srcs when upstream
            # lacks it (e.g., rules_dotnet libraries). Transitive closure
            # follows the upstream edge; direct srcs are always instrumented.
            src_attrs = ["srcs"] + (list(extra_quality_attrs) if extra_quality_attrs else [])
            out.append(coverage_common.instrumented_files_info(
                ctx,
                source_attributes = src_attrs,
                dependency_attributes = ["upstream"],
            ))
        if OutputGroupInfo in upstream:
            out.append(upstream[OutputGroupInfo])
        if RunEnvironmentInfo in upstream:
            out.append(upstream[RunEnvironmentInfo])
        return out
    else:
        fail("dx wrapper: unknown runtime '" + runtime + "': want \"mandatory\" or \"besteffort\"")

def dx_library_forward_rule(provides, required_providers, quality_specs, what, allow_files, upstream_providers, doc, srcs_doc, upstream_doc, extra_attrs = None, runtime = "mandatory", extra_quality_attrs = None):
    """Creates the public forwarding rule for one library wrapper.

    The implementation preserves the required upstream providers plus
    `DefaultInfo`, forwards the shared runtime providers, and adds
    `QualitySourcesInfo` for the direct sources."""

    def _impl(ctx):
        upstream = ctx.attr.upstream
        return (
            dx_preserved_providers(upstream, required_providers, what) +
            [upstream[DefaultInfo]] +
            _dx_runtime_providers(ctx, upstream, what, runtime, extra_quality_attrs) +
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
    """

    def _impl(ctx):
        upstream = ctx.attr.upstream
        return (
            [dx_symlink_default_info(ctx, what)] +
            dx_preserved_providers(upstream, required_providers, what) +
            dx_forwarded_optional(upstream, optional_providers, what) +
            _dx_runtime_providers(ctx, upstream, what, runtime, extra_quality_attrs) +
            [dx_quality_sources(_dx_quality_files(ctx, extra_quality_attrs), quality_specs, str(ctx.label))]
        )

    attrs = dx_forward_attrs(
        allow_files = allow_files,
        srcs_doc = srcs_doc,
        upstream_providers = upstream_providers,
        upstream_doc = upstream_doc,
        extra_attrs = extra_attrs,
    )
    attrs.update(dx_symlink_windows_attr())
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

def dx_binary_forward_kwargs(kwargs):
    """Returns the forwarder kwargs for one binary shape.

    `tags` ride verbatim (binaries keep `manual` filtering on both
    shapes), `aspect_hints` ride the public forwarder where quality
    aspects visit, and `target_compatible_with` rides both shapes so
    an incompatible platform skips the pair together. Remaining kwargs
    stay upstream-only.
    Contract: `docs/quality/quality-sources.md`.
    """
    out = {}
    if kwargs.get("tags", None) != None:
        out["tags"] = kwargs["tags"]
    if kwargs.get("aspect_hints", None) != None:
        out["aspect_hints"] = kwargs["aspect_hints"]
    if kwargs.get("target_compatible_with", None) != None:
        out["target_compatible_with"] = kwargs["target_compatible_with"]
    return out

def dx_test_upstream_kwargs(kwargs, srcs = None):
    """Returns the private upstream kwargs for one test shape.

    `manual` is stripped so both the private upstream and the public
    wrapper run under `//...` (double-execution is the cost of green
    suites); visibility is forced private; `srcs` is set when given.
    Contract: `docs/quality/quality-sources.md`.
    """
    out = dict(kwargs)
    if "tags" in out:
        kept = [t for t in out["tags"] if t != "manual"]
        if len(kept) > 0:
            out["tags"] = kept
        else:
            out.pop("tags")
    out["visibility"] = ["//visibility:private"]
    if srcs != None:
        out["srcs"] = srcs
    return out

def dx_test_forward_kwargs(kwargs):
    """Returns the forwarder kwargs for one test shape.

    Standard test attributes via `dx_forwarded_test_kwargs` plus
    `aspect_hints` (quality aspects visit the public forwarder) and
    `target_compatible_with` (both shapes skip together on an
    incompatible platform). Contract: `docs/quality/quality-sources.md`.
    """
    out = dx_forwarded_test_kwargs(kwargs)
    if kwargs.get("aspect_hints", None) != None:
        out["aspect_hints"] = kwargs["aspect_hints"]
    if kwargs.get("target_compatible_with", None) != None:
        out["target_compatible_with"] = kwargs["target_compatible_with"]
    return out

def dx_wrap_binary(name, upstream_rule, forward_rule, srcs, visibility = None, upstream_kwargs = None, **kwargs):
    """Instantiates one private upstream binary plus its public forwarder.

    `upstream_kwargs`, when given, is the transformed upstream-only base
    (compiler flags, target frameworks, `main_class`); otherwise the
    caller kwargs are the base. Upstream keeps `srcs` only when non-empty
    so thin-entry shapes own no upstream sources, and stays private.
    The forwarder owns `srcs` directly and takes `tags` plus
    `aspect_hints` from the caller kwargs; remaining kwargs stay
    upstream-only. Contract: `docs/quality/quality-sources.md`.
    """
    effective = dict(upstream_kwargs) if upstream_kwargs != None else dict(kwargs)
    if len(srcs) > 0:
        effective["srcs"] = srcs
    elif "srcs" in effective:
        effective.pop("srcs")
    effective["visibility"] = ["//visibility:private"]
    upstream_rule(
        name = name + "_upstream",
        **effective
    )
    forward_rule(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
        **dx_binary_forward_kwargs(kwargs)
    )

def dx_wrap_test(name, upstream_rule, forward_rule, srcs, visibility = None, upstream_kwargs = None, extra_forward_kwargs = None, **kwargs):
    """Instantiates one private upstream test plus its public forwarder.

    `upstream_kwargs`, when given, is the transformed upstream-only base
    (compiler flags, `crate`, entry wiring); otherwise the caller kwargs
    are the base. The upstream side strips `manual`, stays private, and
    takes `srcs` when given. The forwarder takes the standard test
    attributes plus `aspect_hints`, is marked `testonly`, and owns
    `srcs` (empty when the wrapper owns no direct sources).
    `extra_forward_kwargs` carries forwarder-only extras such as the
    mirrored `env_inherit`. Contract: `docs/quality/quality-sources.md`.
    """
    base = dict(upstream_kwargs) if upstream_kwargs != None else dict(kwargs)
    effective = dx_test_upstream_kwargs(base, srcs = srcs)
    forward_srcs = srcs if srcs != None else []
    forward_kwargs = dx_test_forward_kwargs(kwargs)
    if extra_forward_kwargs:
        forward_kwargs.update(extra_forward_kwargs)
    upstream_rule(
        name = name + "_upstream",
        **effective
    )
    forward_rule(
        name = name,
        testonly = True,
        upstream = name + "_upstream",
        srcs = forward_srcs,
        visibility = visibility,
        **forward_kwargs
    )

def dx_wrap(name, upstream_rule, forward_rule, srcs, visibility = None, **kwargs):
    """Instantiates one private upstream target plus its public forwarder.

    `aspect_hints` (typed native-config labels) ride the public forwarder
 where quality aspects visit (lane A): the forwarder is the
    `QualitySourcesInfo` owner, so hints must reach it, not only the
    private upstream. `hdrs` (C/C++ headers) ride both shapes where the
    forwarder owns them for `QualitySourcesInfo`. `tags` ride only the
    forwarder so lane-A filtering stays honest on the visited target and
    target tags never become per-action execution info on the private
    upstream (Bazel 9 folds tags into every owned action, which makes two
    targets that copy the same source file conflict when only one is
    tagged). `testonly` rides both shapes so a testonly dependency edge
    stays legal on the private upstream; `timeout`/`flaky`/`shard_count`/
    `size` are test-rule built-ins and stay upstream-only through `dx_wrap`
    (test forwarders use `dx_forwarded_test_kwargs`). The forwarder
    defaults to private visibility when the caller passes none, so a
    public package default never leaks the forwarder. Remaining kwargs
    stay upstream-only. See issue #928.
    """
    hints = kwargs.get("aspect_hints", None)
    hdrs = kwargs.get("hdrs", None)
    tags = kwargs.pop("tags", None)
    upstream_rule(
        name = name + "_upstream",
        srcs = srcs,
        visibility = ["//visibility:private"],
        **kwargs
    )
    forward_kwargs = {}
    if hints != None:
        forward_kwargs["aspect_hints"] = hints
    if hdrs != None:
        forward_kwargs["hdrs"] = hdrs
    if tags != None:
        forward_kwargs["tags"] = tags
    if kwargs.get("testonly", None) != None:
        forward_kwargs["testonly"] = kwargs["testonly"]
    forward_rule(
        name = name,
        upstream = name + "_upstream",
        srcs = srcs,
        visibility = visibility,
        **forward_kwargs
    )
