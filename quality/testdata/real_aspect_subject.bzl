"""Analysis subject observing real-aspect dx_results (M04 WP2).

`real_aspect_subject` depends on one real fixture target with the real
lint/format aspects applied and republishes the merged `dx_results` file
set as `DxSubjectInfo` fields. `starlark_test` analysis mode pins the
rendering, proving exact capability presence/absence: no generic fallback,
no empty actions, capability tags honored, Vale config-required.
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo")
load("//quality:real_aspects.bzl", "real_format_aspect", "real_lint_aspect")
load("//quality:sources.bzl", "QualitySourcesInfo")

def _label_text(label):
    text = str(label)
    if text.startswith("@@"):  # buildifier: disable=canonical-repository
        return text[2:]
    return text

def _real_aspect_subject_impl(ctx):
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

real_aspect_subject = rule(
    implementation = _real_aspect_subject_impl,
    attrs = {
        "target": attr.label(
            aspects = [real_lint_aspect, real_format_aspect],
            mandatory = True,
            doc = "Real fixture target observed with real capability aspects applied.",
        ),
    },
    doc = "Exposes merged real dx_results basenames for aspect evidence.",
)
