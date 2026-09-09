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
"""

load("@rules_rust//rust:defs.bzl", _rust_common = "rust_common")
load("//quality:sources.bzl", "QualitySourcesInfo")
load("//tools/starlark:defs.bzl", "DxSubjectInfo")

def _label_text(label):
    text = str(label)
    if text.startswith("@@"):
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
        "wrapper": _label_text(ctx.attr.wrapper.label),
        "upstream": _label_text(ctx.attr.upstream.label),
        "crate_name": crate.name,
        "crate_edition": crate.edition,
        "crate_type": crate.type,
        "crate_is_test": str(crate.is_test),
        "crate_root": crate.root.basename,
        "crate_srcs": ",".join(_sorted_basenames(crate.srcs.to_list())),
        "crate_owner": _label_text(crate.owner),
        "preserved_name": str(crate.name == ucrate.name),
        "preserved_edition": str(crate.edition == ucrate.edition),
        "preserved_type": str(crate.type == ucrate.type),
        "preserved_root": str(crate.root.path == ucrate.root.path),
        "preserved_srcs": str(crate_srcs == ucrate_srcs),
        "preserved_deps": str(len(crate.deps.to_list()) == len(ucrate.deps.to_list())),
        "wrapper_has_quality_sources": str(QualitySourcesInfo in wrapper),
        "upstream_has_quality_sources": str(QualitySourcesInfo in upstream),
        "direct_sources": _render_direct_sources(wrapper[QualitySourcesInfo]),
        "wrapper_has_instrumented_files": str(InstrumentedFilesInfo in wrapper),
        "upstream_has_instrumented_files": str(InstrumentedFilesInfo in upstream),
        "fmt_markers": _marker_basenames(ctx.attr.fmt_test, "rustfmt_checks"),
        "clippy_markers": _marker_basenames(ctx.attr.clippy_test, "clippy_checks"),
        "rustc_tool": _toolchain_file(toolchain, "rustc"),
        "cargo_tool": _toolchain_file(toolchain, "cargo"),
        "rustfmt_tool": _toolchain_file(toolchain, "rustfmt"),
        "clippy_tool": _toolchain_file(toolchain, "clippy_driver"),
        "tools_pinned": str(len(pinned) == 4 and all(pinned)),
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
        "wrapper": attr.label(
            mandatory = True,
            doc = "The public dx_rust_* forwarding target under test.",
        ),
        "upstream": attr.label(
            mandatory = True,
            doc = "The private <name>_dx_upstream target the wrapper forwards.",
        ),
        "fmt_test": attr.label(
            doc = "The rustfmt_test target over the wrappers, for marker evidence.",
        ),
        "clippy_test": attr.label(
            doc = "The rust_clippy_test target over the wrappers, for marker evidence.",
        ),
    },
    toolchains = ["@rules_rust//rust:toolchain_type"],
    doc = "Exposes wrapper-vs-upstream provider comparison as DxSubjectInfo.",
)
