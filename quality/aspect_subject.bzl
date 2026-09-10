"""Analysis subject observing aspect-produced dx_results (M03 WP2c).

`aspect_subject` depends on one fixture target with all four capability
aspects applied and republishes the merged `dx_results` file set as
`DxSubjectInfo` fields. `starlark_test` analysis mode pins the rendering,
proving exact capability presence/absence: no generic fallback, no empty
actions, capability tags honored.
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo")
load("//quality:aspects.bzl", "audit_aspect", "format_aspect", "lint_aspect", "typecheck_aspect")
load("//quality:sources.bzl", "QualitySourcesInfo")

def _label_text(label):
    text = str(label)
    if text.startswith("@@"):  # buildifier: disable=canonical-repository
        return text[2:]
    return text

def _aspect_subject_impl(ctx):
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

aspect_subject = rule(
    implementation = _aspect_subject_impl,
    attrs = {
        "target": attr.label(
            aspects = [lint_aspect, format_aspect, typecheck_aspect, audit_aspect],
            mandatory = True,
            doc = "Fixture target observed with all capability aspects applied.",
        ),
    },
    doc = "Exposes merged dx_results basenames for aspect evidence.",
)
