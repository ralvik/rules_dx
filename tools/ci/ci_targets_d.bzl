"""Split from `BUILD.bazel`. No behavior change."""

load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("@rules_shell//shell:sh_test.bzl", "sh_test")
load("//quality:fixtures.bzl", "real_source_target")


def add_d():
    sh_binary(
        name = "shell_env_qualification",
        srcs = ["shell_env_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Bindgen LLVM-22-vs-23 compat harness; see docs/generation/rust.md#binding-generation.
    sh_binary(
        name = "bindgen_qualification",
        srcs = ["bindgen_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # CXX graph identity harness; see docs/testing/verification-matrix.md.
    sh_binary(
        name = "cxx_identity_qualification",
        srcs = ["cxx_identity_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Exact-target discovery harness; see docs/cli/target-resolution.md#exact-target-discovery.
    sh_binary(
        name = "exact_target_qualification",
        srcs = ["exact_target_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # C++ exact-target snapshot harness; see docs/cli/target-resolution.md#exact-target-discovery.
    sh_binary(
        name = "cpp_snapshot_qualification",
        srcs = ["cpp_snapshot_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # JUnit 6.1.3 plus 5.14.x fallback harness; see docs/product/support-matrix.md#provisional-default-test-runners.
    sh_binary(
        name = "junit_qualification",
        srcs = ["junit_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # xUnit v3 runner harness; see docs/product/support-matrix.md#provisional-default-test-runners.
    sh_binary(
        name = "xunit_qualification",
        srcs = ["xunit_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Go test wiring harness; see docs/product/support-matrix.md#provisional-default-test-runners.
    sh_binary(
        name = "gotest_qualification",
        srcs = ["gotest_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # GoogleTest v1.18.0 plus C++17 floor harness; see docs/product/support-matrix.md#provisional-default-test-runners.
    sh_binary(
        name = "googletest_qualification",
        srcs = ["googletest_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # ScalaTest runner harness; see docs/product/support-matrix.md#provisional-default-test-runners.
    sh_binary(
        name = "scalatest_qualification",
        srcs = ["scalatest_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Maven maven_install.json plus fail_if_repin_required harness; see docs/product/support-matrix.md#provisional-default-dependency-locks.
    sh_binary(
        name = "maven_lock_qualification",
        srcs = ["maven_lock_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Paket lock wiring harness; see docs/product/support-matrix.md#provisional-default-dependency-locks.
    sh_binary(
        name = "paket_qualification",
        srcs = ["paket_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Go from_file lock wiring harness; see docs/product/support-matrix.md#provisional-default-dependency-locks.
    sh_binary(
        name = "godeps_qualification",
        srcs = ["godeps_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # C/C++ sha256-integrity plus no-system-package harness; see docs/product/support-matrix.md#provisional-default-dependency-locks.
    sh_binary(
        name = "cc_hermetic_qualification",
        srcs = ["cc_hermetic_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # JVM quality defaults harness; see docs/product/support-matrix.md#provisional-default-quality-tools.
    sh_binary(
        name = "jvm_quality_qualification",
        srcs = ["jvm_quality_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Scala +.NET quality defaults harness; see docs/product/support-matrix.md#provisional-default-quality-tools.
    sh_binary(
        name = "scala_dotnet_defaults_qualification",
        srcs = ["scala_dotnet_defaults_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Native quality defaults harness; see docs/product/support-matrix.md#provisional-default-quality-tools.
    sh_binary(
        name = "native_quality_qualification",
        srcs = ["native_quality_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Structured quality defaults harness; see docs/product/support-matrix.md#provisional-default-quality-tools.
    sh_binary(
        name = "structured_defaults_qualification",
        srcs = ["structured_defaults_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # File-family quality defaults harness; see docs/product/support-matrix.md#provisional-default-quality-tools.
    sh_binary(
        name = "file_family_defaults_qualification",
        srcs = ["file_family_defaults_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Scalafix console + semanticdb-classpath wiring harness; see docs/product/support-matrix.md#provisional-adapter-input-notes.
    sh_binary(
        name = "scalafix_qualification",
        srcs = ["scalafix_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Roslyn per-TFM-RID SARIF aggregation harness; see docs/product/support-matrix.md#provisional-adapter-input-notes.
    sh_binary(
        name = "roslyn_qualification",
        srcs = ["roslyn_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # FSharpLint console vs library-API binding harness; see docs/product/support-matrix.md#provisional-adapter-input-notes.
    sh_binary(
        name = "fsharplint_qualification",
        srcs = ["fsharplint_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Scala plus .NET adapters harness (See: docs/quality/tool-integrations.md#initial-adapter-qualification, issue #797);
    sh_binary(
        name = "scala_dotnet_adapters_qualification",
        srcs = ["scala_dotnet_adapters_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Native adapters harness (See: docs/quality/tool-integrations.md#initial-adapter-qualification, issue #798);
    sh_binary(
        name = "native_adapters_qualification",
        srcs = ["native_adapters_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Structured adapters harness (See: docs/quality/tool-integrations.md#initial-adapter-qualification, issue #799);
    sh_binary(
        name = "structured_adapters_qualification",
        srcs = ["structured_adapters_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Interpreted/file-family adapters harness (See: docs/quality/tool-integrations.md#initial-adapter-qualification, issue #800);
    sh_binary(
        name = "file_family_adapters_qualification",
        srcs = ["file_family_adapters_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Stable-stack compose harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "stable_stack_qualification",
        srcs = ["stable_stack_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Windows immutable-lazy acquisition harness; see docs/native-toolchains.md#windows-acquisition-and-compatibility.
    sh_binary(
        name = "windows_acquisition_qualification",
        srcs = ["windows_acquisition_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Apple plus Microsoft acquisition rights harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "acquisition_rights_qualification",
        srcs = ["acquisition_rights_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Windows EULA acknowledgement UX harness; see docs/native-toolchains.md#windows-acquisition-and-compatibility.
    sh_binary(
        name = "windows_eula_qualification",
        srcs = ["windows_eula_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Per-host laziness plus symlink-privilege harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "laziness_host_qualification",
        srcs = ["laziness_host_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Windows transport plus ABI harness; see docs/native-toolchains.md#windows-acquisition-and-compatibility.
    sh_binary(
        name = "windows_transport_qualification",
        srcs = ["windows_transport_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Prebuilt interop harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "prebuilt_interop_qualification",
        srcs = ["prebuilt_interop_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Linux corpus harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "linux_corpus_qualification",
        srcs = ["linux_corpus_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Deployment plus execution floors harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "deployment_floors_qualification",
        srcs = ["deployment_floors_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # LCOV accounting harness; see docs/native-toolchains.md#coverage-generation-and-ide-gaps.
    sh_binary(
        name = "lcov_accounting_qualification",
        srcs = ["lcov_accounting_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Cargo metadata harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "cargo_metadata_qualification",
        srcs = ["cargo_metadata_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Strict generation harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "strict_generation_qualification",
        srcs = ["strict_generation_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Cross routes harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "cross_routes_qualification",
        srcs = ["cross_routes_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Bounded remediation harness; see docs/native-toolchains.md#qualification-questions-and-delivery.
    sh_binary(
        name = "remediation_bounds_qualification",
        srcs = ["remediation_bounds_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Layer-2 adapter-less plus composition plus depcheck harness; see docs/testing/verification-matrix.md.
    sh_binary(
        name = "layer2_opens_qualification",
        srcs = ["layer2_opens_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Quality core plus result contract harness; see docs/quality/quality-result-protocol.md.
    sh_binary(
        name = "result_contract_qualification",
        srcs = ["result_contract_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Quality family taxonomy harness; see docs/quality/quality-sources.md.
    sh_binary(
        name = "quality_taxonomy_qualification",
        srcs = ["quality_taxonomy_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Python source-audit split harness; see docs/product/support-matrix.md.
    sh_binary(
        name = "python_audit_qualification",
        srcs = ["python_audit_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Strict preset harness; see docs/quality/strict-preset.md.
    sh_binary(
        name = "strict_preset_qualification",
        srcs = ["strict_preset_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Docs build check serve harness; see docs/documentation/build-check-serve.md.
    sh_binary(
        name = "docs_build_qualification",
        srcs = ["docs_build_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Dogfood-freshness battery as a Bazel-owned target (See: tools/ci/dogfood_freshness.sh, issue #926): the
    # CI dogfood step runs `bazel run //tools/ci:dogfood_freshness`, never
    # direct sh, so the battery itself is versioned and reviewable.
    sh_binary(
        name = "dogfood_freshness",
        srcs = ["dogfood_freshness.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Docs/testing gates for issue #926 (See: tools/ci/docs_testing_gates.sh, issue #926; items 1,2,3,4,7,8-env).
    sh_binary(
        name = "docs_testing_gates",
        srcs = ["docs_testing_gates.sh"],
        data = [
            "//tools/sh:bootstrap",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Examples/consumer gates for issue #926 (See: tools/ci/examples_consumer_gates.sh, issue #926; items 6,8-negative,9).
    sh_binary(
        name = "examples_consumer_gates",
        srcs = ["examples_consumer_gates.sh"],
        data = [
            "//tools/sh:bootstrap",
            "//tools/sh:lib",
            "//tools/sh:snapshot",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )
