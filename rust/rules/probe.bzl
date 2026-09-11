"""Conformance subject for the minimal Rust wrappers (M02).

`dx_wrapper_subject` observes one `dx_rust_*` wrapper next to its private
`<name>_dx_upstream` target and exposes the comparison as `DxSubjectInfo`
string fields for `starlark_test` analysis mode. It proves three things the
M02 milestone evidence requires:

- Provider preservation: the wrapper's `CrateInfo` matches the upstream's
  field by field (`preserved_*`), and both carry `InstrumentedFilesInfo`.
- Single source owner: the wrapper reports `QualitySourcesInfo` while the
  private upstream reports none (`upstream_has_quality_sources`).
- No ambient tool discovery: lint markers come from the pinned-toolchain
  `rustfmt_test`/`rust_clippy_test` targets over the wrappers, and the
  resolved `rules_rust` toolchain binaries live under the pinned external
  repository, never on the host `PATH`.

`dx_wrapper_cc_subject` is the same probe for the Cc-linking shapes
(`dx_rust_shared_library`, `dx_rust_static_library`), whose upstream
provides no `CrateInfo`: crate facts are read from the
`TestCrateInfo`-wrapped crate and the `CcInfo` linking surface is pinned
by linker-input count (`cc_linker_inputs`, `preserved_cc_inputs`).
"""

load("@rules_cc//cc/common:cc_info.bzl", "CcInfo")
load("@rules_rust//rust:defs.bzl", _rust_common = "rust_common")
load("//libs/starlark:defs.bzl", "DxSubjectInfo")
load("//quality:sources.bzl", "QualitySourcesInfo")

def _label_text(label):
    text = str(label)
    if text.startswith("@@"):  # buildifier: disable=canonical-repository
        return text[2:]
    return text

def _sorted_basenames(files):
    return sorted([f.basename for f in files])

def _render_direct_sources(info):
    parts = []
    for class_id in sorted(info.direct_sources.keys()):
        parts.append(class_id + ":" + ",".join(_sorted_basenames(info.direct_sources[class_id].to_list())))
    if len(parts) == 0:
        return "(none)"
    return ";".join(parts)

def _marker_basenames(lint_test, group):
    if lint_test == None:
        return "(none)"
    if OutputGroupInfo not in lint_test:
        return "(missing-output-groups)"
    groups = lint_test[OutputGroupInfo]
    if group not in groups:
        return "(missing-group)"
    names = _sorted_basenames(groups[group].to_list())
    if len(names) == 0:
        return "(empty)"
    return ",".join(names)

def _toolchain_file(toolchain, name):
    info = getattr(toolchain, name, None)
    if info == None:
        return "(none)"
    return info.basename

def _dx_wrapper_subject_impl(ctx):
    wrapper = ctx.attr.wrapper
    upstream = ctx.attr.upstream
    crate = wrapper[_rust_common.crate_info]
    ucrate = upstream[_rust_common.crate_info]

    crate_srcs = sorted([f.path for f in crate.srcs.to_list()])
    ucrate_srcs = sorted([f.path for f in ucrate.srcs.to_list()])

    toolchain = ctx.toolchains["@rules_rust//rust:toolchain_type"]
    tool_paths = [getattr(toolchain, name, None) for name in ["rustc", "cargo", "rustfmt", "clippy_driver"]]
    pinned = [t != None and "rules_rust" in t.path for t in tool_paths]

    fields = {
        "cargo_tool": _toolchain_file(toolchain, "cargo"),
        "clippy_markers": _marker_basenames(ctx.attr.clippy_test, "clippy_checks"),
        "clippy_tool": _toolchain_file(toolchain, "clippy_driver"),
        "crate_edition": crate.edition,
        "crate_is_test": str(crate.is_test),
        "crate_name": crate.name,
        "crate_owner": _label_text(crate.owner),
        "crate_root": crate.root.basename,
        "crate_srcs": ",".join(_sorted_basenames(crate.srcs.to_list())),
        "crate_type": crate.type,
        "direct_sources": _render_direct_sources(wrapper[QualitySourcesInfo]),
        "fmt_markers": _marker_basenames(ctx.attr.fmt_test, "rustfmt_checks"),
        "preserved_deps": str(len(crate.deps.to_list()) == len(ucrate.deps.to_list())),
        "preserved_edition": str(crate.edition == ucrate.edition),
        "preserved_name": str(crate.name == ucrate.name),
        "preserved_root": str(crate.root.path == ucrate.root.path),
        "preserved_srcs": str(crate_srcs == ucrate_srcs),
        "preserved_type": str(crate.type == ucrate.type),
        "rustc_tool": _toolchain_file(toolchain, "rustc"),
        "rustfmt_tool": _toolchain_file(toolchain, "rustfmt"),
        "tools_pinned": str(len(pinned) == 4 and all(pinned)),
        "upstream": _label_text(ctx.attr.upstream.label),
        "upstream_has_instrumented_files": str(InstrumentedFilesInfo in upstream),
        "upstream_has_quality_sources": str(QualitySourcesInfo in upstream),
        "wrapper": _label_text(ctx.attr.wrapper.label),
        "wrapper_has_instrumented_files": str(InstrumentedFilesInfo in wrapper),
        "wrapper_has_quality_sources": str(QualitySourcesInfo in wrapper),
    }
    out = ctx.actions.declare_file(ctx.label.name + ".txt")
    ctx.actions.write(out, "\n".join([k + "=" + fields[k] for k in sorted(fields.keys())]) + "\n")
    return [
        DefaultInfo(files = depset([out])),
        DxSubjectInfo(fields = fields),
    ]

dx_wrapper_subject = rule(
    implementation = _dx_wrapper_subject_impl,
    attrs = {
        "clippy_test": attr.label(
            doc = "The rust_clippy_test target over the wrappers, for marker evidence.",
        ),
        "fmt_test": attr.label(
            doc = "The rustfmt_test target over the wrappers, for marker evidence.",
        ),
        "upstream": attr.label(
            mandatory = True,
            doc = "The private <name>_dx_upstream target the wrapper forwards.",
        ),
        "wrapper": attr.label(
            mandatory = True,
            doc = "The public dx_rust_* forwarding target under test.",
        ),
    },
    toolchains = ["@rules_rust//rust:toolchain_type"],
    doc = "Exposes wrapper-vs-upstream provider comparison as DxSubjectInfo.",
)

def _dx_wrapper_cc_subject_impl(ctx):
    wrapper = ctx.attr.wrapper
    upstream = ctx.attr.upstream
    crate = wrapper[_rust_common.test_crate_info].crate
    ucrate = upstream[_rust_common.test_crate_info].crate

    crate_srcs = sorted([f.path for f in crate.srcs.to_list()])
    ucrate_srcs = sorted([f.path for f in ucrate.srcs.to_list()])

    toolchain = ctx.toolchains["@rules_rust//rust:toolchain_type"]
    tool_paths = [getattr(toolchain, name, None) for name in ["rustc", "cargo", "rustfmt", "clippy_driver"]]
    pinned = [t != None and "rules_rust" in t.path for t in tool_paths]

    cc_inputs = len(wrapper[CcInfo].linking_context.linker_inputs.to_list())
    ucc_inputs = len(upstream[CcInfo].linking_context.linker_inputs.to_list())

    fields = {
        "cargo_tool": _toolchain_file(toolchain, "cargo"),
        "cc_linker_inputs": str(cc_inputs),
        "clippy_markers": _marker_basenames(ctx.attr.clippy_test, "clippy_checks"),
        "clippy_tool": _toolchain_file(toolchain, "clippy_driver"),
        "crate_edition": crate.edition,
        "crate_is_test": str(crate.is_test),
        "crate_name": crate.name,
        "crate_owner": _label_text(crate.owner),
        "crate_root": crate.root.basename,
        "crate_srcs": ",".join(_sorted_basenames(crate.srcs.to_list())),
        "crate_type": crate.type,
        "direct_sources": _render_direct_sources(wrapper[QualitySourcesInfo]),
        "fmt_markers": _marker_basenames(ctx.attr.fmt_test, "rustfmt_checks"),
        "preserved_cc_inputs": str(cc_inputs == ucc_inputs),
        "preserved_deps": str(len(crate.deps.to_list()) == len(ucrate.deps.to_list())),
        "preserved_edition": str(crate.edition == ucrate.edition),
        "preserved_name": str(crate.name == ucrate.name),
        "preserved_root": str(crate.root.path == ucrate.root.path),
        "preserved_srcs": str(crate_srcs == ucrate_srcs),
        "preserved_type": str(crate.type == ucrate.type),
        "rustc_tool": _toolchain_file(toolchain, "rustc"),
        "rustfmt_tool": _toolchain_file(toolchain, "rustfmt"),
        "tools_pinned": str(len(pinned) == 4 and all(pinned)),
        "upstream": _label_text(ctx.attr.upstream.label),
        "upstream_has_instrumented_files": str(InstrumentedFilesInfo in upstream),
        "upstream_has_quality_sources": str(QualitySourcesInfo in upstream),
        "wrapper": _label_text(ctx.attr.wrapper.label),
        "wrapper_has_instrumented_files": str(InstrumentedFilesInfo in wrapper),
        "wrapper_has_quality_sources": str(QualitySourcesInfo in wrapper),
    }
    out = ctx.actions.declare_file(ctx.label.name + ".txt")
    ctx.actions.write(out, "\n".join([k + "=" + fields[k] for k in sorted(fields.keys())]) + "\n")
    return [
        DefaultInfo(files = depset([out])),
        DxSubjectInfo(fields = fields),
    ]

dx_wrapper_cc_subject = rule(
    implementation = _dx_wrapper_cc_subject_impl,
    attrs = {
        "clippy_test": attr.label(
            doc = "The rust_clippy_test target over the wrappers, for marker evidence.",
        ),
        "fmt_test": attr.label(
            doc = "The rustfmt_test target over the wrappers, for marker evidence.",
        ),
        "upstream": attr.label(
            mandatory = True,
            providers = [[_rust_common.test_crate_info]],
            doc = "The private <name>_dx_upstream Cc-linking target the wrapper forwards.",
        ),
        "wrapper": attr.label(
            mandatory = True,
            providers = [[_rust_common.test_crate_info]],
            doc = "The public dx_rust_shared_library/dx_rust_static_library target under test.",
        ),
    },
    toolchains = ["@rules_rust//rust:toolchain_type"],
    doc = "Exposes Cc-wrapper-vs-upstream provider comparison as DxSubjectInfo.",
)
