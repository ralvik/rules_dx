"""Aspect-lint golden harness (issue #406).

`lint_test` proves one fixture's real-pipeline findings as data: a
`real_source_target` plus `real_aspect_subject` (cf.
`quality/testdata/real_aspect_subject.bzl`) exposing the merged
`dx_results` `OutputGroup` (`quality/real_aspects.bzl`), pinned by a
`starlark_test` analysis assertion over `DxSubjectInfo`. Findings become
data; only golden-mismatch fails. Dirty inputs never live here as
aspect subjects: they ride the Layer-2 matrix (`runner_matrix_cases.bzl`)
as provider-less `srcs`/`generated` bytes with `print_result` goldens,
or `tools/depcheck/testdata/`-style filegroups, so `dx lint --check //...`
stays green while `bazel test //...` proves bad is correctly caught.

Hermeticity: sandboxed, cacheable, no nested Bazel, no `manual`/`local`/
`exclusive`/`no-sandbox`. Offline via the standard aspect action inputs;
`TEST_TMPDIR` scratch is owned by the underlying runner/matrix harnesses.
"""

load("//libs/starlark:defs.bzl", "starlark_test")
load("//quality:fixtures.bzl", "real_source_target")
load("//quality/testdata:real_aspect_subject.bzl", "real_aspect_subject")

def lint_test(name, srcs, expected_observations, **kwargs):
    """Proves one lint/format subject's `dx_results` shape as data.

    Creates a private `real_source_target` (`<name>_fixture`) from `srcs`,
    a `real_aspect_subject` (`<name>_subject`) over it, and a passing
    `starlark_test` (`<name>`) diffing the subject's `DxSubjectInfo`
    against `expected_observations`. Only golden-mismatch fails; the
    fixture itself never fails the build.

    For dirty (expected-failure) coverage, prefer the matrix
    (`runner_matrix_cases.bzl`): provider-less bytes with `print_result`
    goldens, both `*_pass`/`*_fail` green. This macro is for clean
    presence pins; dirty live subjects break `dx <check> --check //...`
    and belong in the matrix, not here.

    Args:
      name: passing test target name (fixture is `<name>_fixture`,
        subject is `<name>_subject`).
      srcs: dict of `real_source_target` srcs kwargs (for example
        `{"python_srcs": ["clean.py"]}`).
      expected_observations: golden `DxSubjectInfo` rendering for the
        single subject.
      **kwargs: extra attributes forwarded to `starlark_test` (size, tags).
    """
    real_source_target(
        name = name + "_fixture",
        **srcs
    )
    real_aspect_subject(
        name = name + "_subject",
        target = ":" + name + "_fixture",
    )
    kwargs.setdefault("size", "small")
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":" + name + "_subject"],
        expected_observations = expected_observations,
        **kwargs
    )
