"""Consumer-workspace shim: re-export the real capability aspects.

`dx format`/`dx lint`/`dx typecheck` pass
`--aspects=//quality:real_aspects.bzl%...`, which resolves in the
consumer (child) workspace. This file re-exports the authoritative
aspects from `@rules_dx` so the child run exercises the real
format pipeline (Buildifier stage over `QualitySourcesInfo`) instead
of a stub. Attr defaults inside the real aspects resolve to the
defining module (`@rules_dx`), so no tool labels leak to the child.
"""

load(
    "@rules_dx//quality:real_aspects.bzl",
    _real_format_aspect = "real_format_aspect",
    _real_lint_aspect = "real_lint_aspect",
    _real_typecheck_aspect = "real_typecheck_aspect",
)

real_format_aspect = _real_format_aspect
real_lint_aspect = _real_lint_aspect
real_typecheck_aspect = _real_typecheck_aspect
