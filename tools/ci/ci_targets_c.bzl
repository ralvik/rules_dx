"""CI hygiene targets shard C (split from `BUILD.bazel`).

Contract: `docs/github-ci.md`.
"""

load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("@rules_shell//shell:sh_test.bzl", "sh_test")


def add_c():
    sh_binary(
        name = "env_codegen_qualification",
        srcs = ["env_codegen_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Offline/airgap bootstrap harness; see docs/deploy/offline-bootstrap.md.
    sh_binary(
        name = "offline_airgap_qualification",
        srcs = ["offline_airgap_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Env plugin-model plus Go cgo exception harness; see docs/environments/environment.md.
    sh_binary(
        name = "env_plugins_cgo_qualification",
        srcs = ["env_plugins_cgo_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Docs-pipeline harness; see tools/ci/docs_pipeline_qualification.sh.
    sh_binary(
        name = "docs_pipeline_qualification",
        srcs = ["docs_pipeline_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Consumer-CI harness; see tools/ci/consumer_ci_qualification.sh.
    sh_binary(
        name = "consumer_ci_qualification",
        srcs = ["consumer_ci_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Review-thread limit plus accounting harness; see docs/github-ci.md#thread-lifecycle-and-limits.
    sh_binary(
        name = "review_threads_qualification",
        srcs = ["review_threads_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # File-family harness; see tools/ci/file_family_qualification.sh.
    sh_binary(
        name = "file_family_qualification",
        srcs = ["file_family_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Helpers harness; see tools/ci/helper_qualification.sh.
    sh_binary(
        name = "helper_qualification",
        srcs = ["helper_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Clap tokenizer harness; see tools/ci/clap_tokenizer_qualification.sh.
    sh_binary(
        name = "clap_tokenizer_qualification",
        srcs = ["clap_tokenizer_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Non-dogfed harness (; hermetic pins under); see tools/ci/non_dogfed_paths.sh.
    sh_binary(
        name = "non_dogfed_paths",
        srcs = ["non_dogfed_paths.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Non-dogfed qualification; see tools/ci/non_dogfed_qualification.sh.
    sh_binary(
        name = "non_dogfed_qualification",
        srcs = ["non_dogfed_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Visibility hardening guard; see docs/contributing/build-conventions.md#visibility.
    sh_binary(
        name = "visibility_guards",
        srcs = ["visibility_guards.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # CLI-contract registry plus behavior harness; see docs/cli/cli-contract.md.
    sh_binary(
        name = "cli_contract_qualification",
        srcs = ["cli_contract_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Migrate syntax plus manifest-selection harness; see docs/cli/commands/migrate.md.
    sh_binary(
        name = "migrate_qualification",
        srcs = ["migrate_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Warnings-as-errors harness; see tools/ci/warnings_as_errors.sh.
    sh_binary(
        name = "warnings_as_errors",
        srcs = ["warnings_as_errors.sh"],
        data = [
            "//tools/sh:bootstrap",
            "//tools/sh:guards",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Signing stack + distribution harness (live successor to
    # closed //); see docs/deploy/release-runbook.md.
    sh_binary(
        name = "signing_distribution_qualification",
        srcs = ["signing_distribution_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Hello-smoke-as-test harness; see docs/testing/verification-matrix.md.
    sh_binary(
        name = "hello_smoke_qualification",
        srcs = ["hello_smoke_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Parser-sample backfill harness; see docs/testing/verification-matrix.md.
    sh_binary(
        name = "parser_sample_qualification",
        srcs = ["parser_sample_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Close-out battery plus docs-gate harness; see docs/testing/verification-matrix.md.
    sh_binary(
        name = "closeout_battery_qualification",
        srcs = ["closeout_battery_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Rustfmt crate-edition harness; see docs/testing/verification-matrix.md.
    sh_binary(
        name = "rustfmt_edition_qualification",
        srcs = ["rustfmt_edition_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Rust library extraction harness; see docs/decisions/0023-rust-libraries-internal.md.
    sh_binary(
        name = "rust_library_qualification",
        srcs = ["rust_library_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Selective update per-set harness; see docs/decisions/0024-selective-update.md.
    sh_binary(
        name = "selective_update_qualification",
        srcs = ["selective_update_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Selective Cargo per-crate wont-fix harness; see docs/cli/commands/audit-update-bazel.md#dx-update.
    sh_binary(
        name = "selective_cargo_qualification",
        srcs = ["selective_cargo_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Selective NuGet per-package wont-fix harness; see docs/cli/commands/audit-update-bazel.md#dx-update.
    sh_binary(
        name = "selective_nuget_qualification",
        srcs = ["selective_nuget_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Selective Maven per-artifact wont-fix harness (issue #634; See: docs/decisions/0024-selective-update.md).
    sh_binary(
        name = "selective_maven_qualification",
        srcs = ["selective_maven_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Selective Go per-module wont-fix harness; see docs/cli/commands/audit-update-bazel.md#dx-update.
    sh_binary(
        name = "selective_go_qualification",
        srcs = ["selective_go_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Bump then update chaining harness (issue #638; See: docs/cli/commands/audit-update-bazel.md#dx-bump).
    sh_binary(
        name = "bump_chain_qualification",
        srcs = ["bump_chain_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Bump discovery outdated enumeration harness (issue #639; See: docs/cli/commands/audit-update-bazel.md#dx-bump).
    sh_binary(
        name = "bump_discovery_qualification",
        srcs = ["bump_discovery_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Bump GHA tag-to-SHA auto resolution harness (issue #640; See: docs/cli/commands/audit-update-bazel.md#dx-bump).
    sh_binary(
        name = "bump_gha_qualification",
        srcs = ["bump_gha_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Update mutation-event wont-fix harness; see docs/cli/output-protocol.md#mutation.
    sh_binary(
        name = "update_events_qualification",
        srcs = ["update_events_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Correlation plus committed-change manifest harness (issue #811; See: docs/cli/output-protocol.md#mutation).
    sh_binary(
        name = "correlation_manifest_qualification",
        srcs = ["correlation_manifest_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Atomic update rollback-plan harness — See: docs/cli/commands/audit-update-bazel.md#dx-update (issue #772).
    sh_binary(
        name = "update_rollback_qualification",
        srcs = ["update_rollback_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Starlark testing futures harness; see docs/testing/starlark.md#future-not-implemented.
    sh_binary(
        name = "starlark_futures_qualification",
        srcs = ["starlark_futures_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # CLI execution/reporting gaps harness; see docs/cli/cli-contract.md.
    sh_binary(
        name = "cli_execution_gaps_qualification",
        srcs = ["cli_execution_gaps_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # CLI strict parsing plus help-goldens harness; see docs/cli/cli-contract.md.
    sh_binary(
        name = "cli_strict_qualification",
        srcs = ["cli_strict_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Build-profile flags plus forwarding harness; see docs/cli/commands/build-test-coverage.md#build-profiles.
    sh_binary(
        name = "build_profiles_qualification",
        srcs = ["build_profiles_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Audit curator declared-inputs harness; see docs/cli/commands/audit-update-bazel.md.
    sh_binary(
        name = "audit_declared_inputs_qualification",
        srcs = ["audit_declared_inputs_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Layer-4 loss restore-or-wont-fix harness; see docs/testing/verification-matrix.md#layers.
    sh_binary(
        name = "layer4_loss_qualification",
        srcs = ["layer4_loss_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # CC opt-out linker harness; see docs/testing/verification-matrix.md.
    sh_binary(
        name = "cc_optout_qualification",
        srcs = ["cc_optout_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Shell-env default vs annotation extension harness; see docs/generation/rust.md#build-scripts.
