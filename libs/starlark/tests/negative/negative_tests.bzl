load("@rules_shell//shell:sh_test.bzl", "sh_test")
load("//libs/starlark:failure_test.bzl", "failure_test")

def failing_check_demo(name):
    sh_test(
        name = name,
        srcs = ["failing_check_test.sh"],
        data = ["//tools/sh:lib"],
        size = "small",
        timeout = "short",
        target_compatible_with = ["@platforms//os:linux"],
    )

def missing_observation_demo(name):
    sh_test(
        name = name,
        srcs = ["missing_observation_test.sh"],
        data = [
            "//tools/sh:lib",
            ":negative_subject",
        ],
        size = "small",
        timeout = "short",
        target_compatible_with = ["@platforms//os:linux"],
    )

def missing_fragment_demo(name):
    sh_test(
        name = name,
        srcs = ["missing_fragment_test.sh"],
        data = [
            "//tools/sh:lib",
            ":present_fixture.txt",
        ],
        size = "small",
        timeout = "short",
        target_compatible_with = ["@platforms//os:linux"],
    )

def _wrong_phase_subject_impl(ctx):
    if len(ctx.attr.subjects) != 0:
        fail("starlark_test (load mode): subjects must be empty: " +
             "load tests observe loading, not analysis")
    return [DefaultInfo(files = depset([]))]

_wrong_phase_subject = rule(
    implementation = _wrong_phase_subject_impl,
    attrs = {
        "subjects": attr.label_list(
        ),
    },
)

def wrong_phase_demo(name):
    _wrong_phase_subject(
        name = name + "_subject",
        subjects = [":negative_subject"],
        tags = ["manual"],
    )
    failure_test(
        name = name,
        target = ":" + name + "_subject",
        expected_failure_substring = "starlark_test (load mode): subjects must be empty",
    )
