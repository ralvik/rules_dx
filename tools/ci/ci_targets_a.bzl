"""CI hygiene targets shard A (split from `BUILD.bazel`).

Contract: `docs/github-ci.md`.
"""

load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("@rules_shell//shell:sh_test.bzl", "sh_test")


def add_a():
    sh_binary(
        name = "corpus_audit",
        srcs = ["corpus_audit.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Green hermetic negative proofs; see docs/product/support-matrix.md for status.

    # Consumer CI template harness (item 1); see tools/ci/consumer_scheduling_test.sh.
    sh_test(
        name = "consumer_scheduling_test",
        size = "small",
        timeout = "short",
        srcs = ["consumer_scheduling_test.sh"],
        args = ["$(rootpath //.github:workflows/reusable-consumer.yml)"],
        data = [
            "//.github:workflows/reusable-consumer.yml",
            "//tools/sh:bootstrap",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    sh_test(
        name = "consumer_aggregate_test",
        size = "small",
        timeout = "short",
        srcs = ["consumer_aggregate_test.sh"],
        args = ["$(rootpath //.github:workflows/reusable-consumer.yml)"],
        data = [
            "//.github:workflows/reusable-consumer.yml",
            "//tools/sh:bootstrap",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    sh_test(
        name = "consumer_guards_test",
        size = "small",
        timeout = "short",
        srcs = ["consumer_guards_test.sh"],
        args = [
            "$(rootpath //.github:workflows/reusable-consumer.yml)",
            "$(rootpath //examples:consumer-ci/caller.yml)",
        ],
        data = [
            "//.github:workflows/reusable-consumer.yml",
            "//examples:consumer-ci/caller.yml",
            "//tools/sh:bootstrap",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    sh_test(
        name = "consumer_pins_test",
        size = "small",
        timeout = "short",
        srcs = ["consumer_pins_test.sh"],
        args = [
            "$(rootpath //.github:workflows/reusable-consumer.yml)",
            "$(rootpath //examples:consumer-ci/caller.yml)",
            "$(rootpath //:MODULE.bazel)",
            "$(rootpath //.github/actions/setup-checkout-bazelisk:action.yml)",
        ],
        data = [
            "//:MODULE.bazel",
            "//.github:workflows/reusable-consumer.yml",
            "//.github/actions/setup-checkout-bazelisk:action.yml",
            "//examples:consumer-ci/caller.yml",
            "//tools/sh:bootstrap",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Example caller pin-sync harness; see tools/ci/examples_pins_test.sh.
    sh_test(
        name = "examples_pins_test",
        size = "small",
        timeout = "short",
        srcs = ["examples_pins_test.sh"],
        args = [
            "$(rootpath //examples:consumer-ci/caller.yml)",
            "$(rootpath //examples:docs-ci/caller.yml)",
            "$(rootpath //:MODULE.bazel)",
        ],
        data = [
            "//:MODULE.bazel",
            "//examples:consumer-ci/caller.yml",
            "//examples:docs-ci/caller.yml",
            "//tools/sh:bootstrap",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Pin single-source consistency; see tools/ci/pin_consistency.sh.
    sh_test(
        name = "pin_consistency_test",
        size = "small",
        timeout = "short",
        srcs = ["pin_consistency.sh"],
        args = [
            "$(rootpath //:.bazelversion)",
            "$(rootpath //:MODULE.bazel)",
            "$(rootpath //tools/bazelrc:src/lib.rs)",
            "$(rootpath //.devcontainer:Dockerfile.prebuilt)",
            "$(rootpath //libs/testing:tested_stack.bzl)",
            "$(rootpath //.github/actions/setup-bazelisk:action.yml)",
            "$(rootpath //docs:contributing/local-workflows.md)",
            "$(rootpath //third_party/go:go.mod)",
            "$(rootpath //:package.json)",
            "$(rootpath //quality/tools/javascript:package.json)",
            "$(rootpath //modules:rust.bzl)",
            "$(rootpath //modules:python.bzl)",
            "$(rootpath //modules:js.bzl)",
            "$(rootpath //modules:java-scala-kotlin.bzl)",
            "$(rootpath //modules:dotnet.bzl)",
            "$(rootpath //modules:npm-uv-crates.bzl)",
            "$(rootpath //modules:toolchains.bzl)",
            "$(rootpath //quality/artifacts:repos.bzl)",
            "$(rootpath //tools/bazelrc:preset.bazelrc)",
            "$(rootpath //:.bazelrc)",
            "$(rootpath //cli/adopt:src/version.rs)",
            "$(rootpath //.github:workflows/ghcr.yml)",
            "$(rootpath //quality/tools/jvm:repos.bzl)",
        ],
        data = [
            "//:.bazelversion",
            "//:.bazelrc",
            "//:MODULE.bazel",
            "//:package.json",
            "//.devcontainer:Dockerfile.prebuilt",
            "//.github:workflows/ghcr.yml",
            "//.github/actions/setup-bazelisk:action.yml",
            "//cli/adopt:src/version.rs",
            "//docs:contributing/local-workflows.md",
            "//libs/testing:tested_stack.bzl",
            "//modules:dotnet.bzl",
            "//modules:java-scala-kotlin.bzl",
            "//modules:js.bzl",
            "//modules:npm-uv-crates.bzl",
            "//modules:python.bzl",
            "//modules:rust.bzl",
            "//modules:toolchains.bzl",
            "//quality/artifacts:repos.bzl",
            "//quality/tools/javascript:package.json",
            "//quality/tools/jvm:repos.bzl",
            "//third_party/go:go.mod",
            "//tools/bazelrc:preset.bazelrc",
            "//tools/bazelrc:src/lib.rs",
            "//tools/sh:bootstrap",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Config single-source consistency (See: tools/ci/config_consistency.sh, issue #912);
    sh_test(
        name = "config_consistency_test",
        size = "small",
        timeout = "short",
        srcs = ["config_consistency.sh"],
        args = [
            "$(rootpath //:biome.json)",
            "$(rootpath //quality/testdata:biome_cfg/biome.json)",
            "$(rootpath //quality/artifacts:update.py)",
            "$(rootpath //:ruff.toml)",
            "$(rootpath //quality/testdata:ruff.toml)",
            "$(rootpath //:package.json)",
            "$(rootpath //quality/tools/javascript:package.json)",
            "$(rootpath //:pnpm-workspace.yaml)",
            "$(rootpath //quality/tools/javascript:pnpm-workspace.yaml)",
            "$(rootpath //quality/tools/python:pyproject.toml)",
            "$(rootpath //quality/tools/python:uv.lock)",
            "$(rootpath //quality:corpus_styles/Dx/Markers.yml)",
            "$(rootpath //quality/testdata:styles/Dx/Markers.yml)",
            "$(rootpath //quality:corpus_vale.ini)",
            "$(rootpath //quality/testdata:vale_test.ini)",
        ],
        data = [
            "//:biome.json",
            "//:package.json",
            "//:pnpm-workspace.yaml",
            "//:ruff.toml",
            "//quality:corpus_styles/Dx/Markers.yml",
            "//quality:corpus_vale.ini",
            "//quality/artifacts:update.py",
            "//quality/testdata:biome_cfg/biome.json",
            "//quality/testdata:ruff.toml",
            "//quality/testdata:styles/Dx/Markers.yml",
            "//quality/testdata:vale_test.ini",
            "//quality/tools/javascript:package.json",
            "//quality/tools/javascript:pnpm-workspace.yaml",
            "//quality/tools/python:pyproject.toml",
            "//quality/tools/python:uv.lock",
            "//tools/sh:bootstrap",
            "//tools/sh:guards",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Ignore-list parity; see tools/ci/ignore_parity.sh.
    sh_test(
        name = "ignore_parity_test",
        size = "small",
        timeout = "short",
        srcs = ["ignore_parity.sh"],
        args = [
            "$(rootpath //:.gitignore)",
            "$(rootpath //:.bazelignore)",
            "$(rootpath //:.bazelrc)",
            "$(rootpath //docs:contributing/local-workflows.md)",
        ],
        data = [
            "//:.bazelignore",
            "//:.bazelrc",
            "//:.gitignore",
            "//docs:contributing/local-workflows.md",
            "//tools/sh:bootstrap",
            "//tools/sh:guards",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Seed-cell coverage gate harness (item 1); see tools/ci/coverage_cell.sh.
    sh_binary(
        name = "coverage_cell",
        srcs = ["coverage_cell.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # LCOV excludes-budget harness (See: tools/ci/coverage_excludes_qualification.sh, issue #1055).
    sh_binary(
        name = "coverage_excludes_qualification",
        srcs = ["coverage_excludes_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Coverage-report guards; see tools/ci/coverage_report_guards.sh.
    sh_binary(
        name = "coverage_report_guards",
        srcs = ["coverage_report_guards.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Target-tag semantics harness (item 3); see tools/ci/target_tags.sh.
    sh_binary(
        name = "target_tags",
        srcs = ["target_tags.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Release-hygiene harness; see tools/ci/release_hygiene.sh.
    sh_binary(
        name = "release_hygiene",
        srcs = ["release_hygiene.sh"],
        data = [
            "//tools/sh:bootstrap",
            "//tools/sh:guards",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Release-policy gate harness (item 2); see tools/ci/release_policy.sh.
    sh_binary(
        name = "release_policy",
        srcs = ["release_policy.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Quality-adapter parity harness; see tools/ci/quality_adapters_parity.sh.
    sh_binary(
        name = "quality_adapters_parity",
        srcs = ["quality_adapters_parity.sh"],
        data = [
            "//tools/sh:bootstrap",
            "//tools/sh:guards",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # JVM-cohort qualification harness; see tools/ci/jvm_cohort_qualification.sh.
    sh_binary(
        name = "jvm_cohort_qualification",
        srcs = ["jvm_cohort_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Scala/.NET-cohort qualification harness; see tools/ci/scala_dotnet_cohort_qualification.sh.
    sh_binary(
        name = "scala_dotnet_cohort_qualification",
        srcs = ["scala_dotnet_cohort_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Native-cohort qualification harness; see tools/ci/native_cohort_qualification.sh.
    sh_binary(
        name = "native_cohort_qualification",
        srcs = ["native_cohort_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Structured-cohort qualification harness; see tools/ci/structured_cohort_qualification.sh.
    sh_binary(
        name = "structured_cohort_qualification",
        srcs = ["structured_cohort_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Interpreted/file-family qualification harness; see tools/ci/interpreted_file_cohort_qualification.sh.
    sh_binary(
        name = "interpreted_file_cohort_qualification",
        srcs = ["interpreted_file_cohort_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Publication-trust harness; see tools/ci/publish_trust.sh.
    sh_binary(
        name = "publish_trust",
        srcs = ["publish_trust.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Code-ownership harness; see tools/ci/code_ownership.sh.
    sh_binary(
        name = "code_ownership",
        srcs = ["code_ownership.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Examples README harness (slice 1); see tools/ci/examples_readme.sh.
    sh_binary(
        name = "examples_readme",
        srcs = ["examples_readme.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Examples laziness harness (slice 2); see tools/ci/examples_laziness.sh.
    sh_binary(
        name = "examples_laziness",
        srcs = ["examples_laziness.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Examples laziness query proof (slice 3); see tools/ci/examples_laziness_query.sh.
    sh_binary(
        name = "examples_laziness_query",
        srcs = ["examples_laziness_query.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Examples laziness aquery proof (slice 4); see tools/ci/examples_laziness_aquery.sh.
    sh_binary(
        name = "examples_laziness_aquery",
        srcs = ["examples_laziness_aquery.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Examples laziness runtime proof (slice 5); see tools/ci/examples_laziness_runtime.sh.
    sh_binary(
        name = "examples_laziness_runtime",
        srcs = ["examples_laziness_runtime.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Laziness analysis guard (zero-delta fetches/actions plus budget); see tools/ci/laziness_analysis_guard.sh.
    sh_binary(
        name = "laziness_analysis_guard",
        srcs = ["laziness_analysis_guard.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Registry-singularity harness; see tools/ci/registry_singularity.sh.
