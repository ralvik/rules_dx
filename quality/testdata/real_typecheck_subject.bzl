"""Analysis subject observing the real typecheck dx_results (WP3).
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo")
load("//libs/starlark:canonical.bzl", "strip_canonical")
load("//quality:real_aspects.bzl", "real_rust_typecheck_aspect", "real_typecheck_aspect")
load("//quality:sources.bzl", "QualitySourcesInfo")

def _label_text(label):
    return strip_canonical(str(label))

def _real_typecheck_subject_impl(ctx):
    target = ctx.attr.target
    dx_files = []
    if OutputGroupInfo in target:
        groups = target[OutputGroupInfo]
        if "dx_results" in groups:
            dx_files = sorted(
                groups["dx_results"].to_list(),
                key = lambda f: f.basename,
            )
    basenames = [f.basename for f in dx_files]
    return [
        DefaultInfo(files = depset([])),
        OutputGroupInfo(dx_results = depset(dx_files)),
        DxSubjectInfo(fields = {
            "dx_count": str(len(basenames)),
            "dx_results": ",".join(basenames) if len(basenames) > 0 else "(none)",
            "has_quality_sources": str(QualitySourcesInfo in target),
            "label": _label_text(ctx.attr.target.label),
        }),
    ]

real_typecheck_subject = rule(
    implementation = _real_typecheck_subject_impl,
    attrs = {
        "target": attr.label(
            aspects = [real_typecheck_aspect, real_rust_typecheck_aspect],
            mandatory = True,
            doc = "Real fixture target observed with the real typecheck aspect applied.",
        ),
    },
    doc = "Exposes merged real typecheck dx_results basenames for aspect evidence.",
)
