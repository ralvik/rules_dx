"""Split from `BUILD.bazel`. No behavior change."""

load("@rules_shell//shell:sh_binary.bzl", "sh_binary")
load("@rules_shell//shell:sh_test.bzl", "sh_test")


def add_b():
    sh_binary(
        name = "registry_singularity",
        srcs = ["registry_singularity.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Backlog-contracts harness; see tools/ci/backlog_contracts.sh.
    sh_binary(
        name = "backlog_contracts",
        srcs = ["backlog_contracts.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Dx facade harness; see tools/ci/dx_facade_qualification.sh.
    sh_binary(
        name = "dx_facade_qualification",
        srcs = ["dx_facade_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Quality cache proof; see tools/ci/quality_cache_aquery.sh.
    sh_binary(
        name = "quality_cache_aquery",
        srcs = ["quality_cache_aquery.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Action execution plus disk-cache harness; see tools/ci/action_execution_cache_qualification.sh.
    sh_binary(
        name = "action_execution_cache_qualification",
        srcs = ["action_execution_cache_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Dependency-check contract harness (; opens under); see tools/ci/depcheck_contract.sh.
    sh_binary(
        name = "depcheck_contract",
        srcs = ["depcheck_contract.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Audit/update/depcheck guards; see tools/ci/audit_update_guards.sh.
    sh_binary(
        name = "audit_update_guards",
        srcs = ["audit_update_guards.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Product Python/shell to Rust boundary guards; see tools/ci/product_runtime_guards.sh.
    sh_binary(
        name = "product_runtime_guards",
        srcs = ["product_runtime_guards.sh"],
        data = [
            "//tools/sh:bootstrap",
            "//tools/sh:guards",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Wrapper sources harness; see tools/ci/wrapper_sources.sh.
    sh_binary(
        name = "wrapper_sources",
        srcs = ["wrapper_sources.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Foundation-mapping guards (ADR 0019); see tools/ci/foundation_maps.sh.
    sh_binary(
        name = "foundation_maps",
        srcs = ["foundation_maps.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Test-only fixture layout guards; see tools/ci/fixture_layout.sh.
    sh_binary(
        name = "fixture_layout",
        srcs = ["fixture_layout.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # GHCR prebuilt-image hygiene (live successor to closed); see tools/ci/ghcr_hygiene.sh.
    sh_binary(
        name = "ghcr_hygiene",
        srcs = ["ghcr_hygiene.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Publication-group guards; see tools/ci/ghcr_publish_guards.sh.
    sh_binary(
        name = "ghcr_publish_guards",
        srcs = ["ghcr_publish_guards.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Quality-foundations guards; see tools/ci/quality_foundations_guards.sh.
    sh_binary(
        name = "quality_foundations_guards",
        srcs = ["quality_foundations_guards.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Widen/update loop contract; see tools/ci/widen_update_loop.sh.
    sh_binary(
        name = "widen_update_loop",
        srcs = ["widen_update_loop.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # BUILD comment hygiene; see tools/ci/build_hygiene.sh.
    sh_binary(
        name = "build_hygiene",
        srcs = ["build_hygiene.sh"],
        data = [
            "//tools/sh:bootstrap",
            "//tools/sh:guards",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Source comment hygiene; see tools/ci/source_hygiene.sh.
    sh_binary(
        name = "source_hygiene",
        srcs = ["source_hygiene.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Distribution/closeout guards; see tools/ci/distribution_closeout_guards.sh.
    sh_binary(
        name = "distribution_closeout_guards",
        srcs = ["distribution_closeout_guards.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Backlog/automation guards; see tools/ci/backlog_automation_guards.sh.
    sh_binary(
        name = "backlog_automation_guards",
        srcs = ["backlog_automation_guards.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Shell contract guard; see tools/ci/shell_contract.sh.
    sh_binary(
        name = "shell_contract",
        srcs = ["shell_contract.sh"],
        data = [
            "//tools/sh:bootstrap",
            "//tools/sh:guards",
            "//tools/sh:lib",
        ],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Supported-evidence gate; see tools/ci/supported_evidence_gate.sh.
    sh_binary(
        name = "supported_evidence_gate",
        srcs = ["supported_evidence_gate.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Promotion-checklist harness; see docs/product/promotion-checklist.md.
    sh_binary(
        name = "promotion_checklist_qualification",
        srcs = ["promotion_checklist_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # SBOM-upload harness; see docs/deploy/release-runbook.md.
    sh_binary(
        name = "sbom_upload_qualification",
        srcs = ["sbom_upload_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Release-arm64 harness; see docs/deploy/release-runbook.md.
    sh_binary(
        name = "release_arm64_qualification",
        srcs = ["release_arm64_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Release-musl harness; see docs/deploy/release-runbook.md.
    sh_binary(
        name = "release_musl_qualification",
        srcs = ["release_musl_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Release-macos-arm64 harness; see docs/deploy/release-runbook.md.
    sh_binary(
        name = "release_macos_arm64_qualification",
        srcs = ["release_macos_arm64_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Release-windows harness; see docs/deploy/release-runbook.md.
    sh_binary(
        name = "release_windows_qualification",
        srcs = ["release_windows_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Hermetic CLI-contract coverage; see docs/testing/verification-matrix.md.

    # Coverage-spill containment harness; see tools/ci/coverage_spill.sh.
    sh_binary(
        name = "coverage_spill",
        srcs = ["coverage_spill.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Coverage/remote harness; see tools/ci/coverage_qualification.sh.
    sh_binary(
        name = "coverage_qualification",
        srcs = ["coverage_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Musl harness; see tools/ci/musl_qualification.sh.
    sh_binary(
        name = "musl_qualification",
        srcs = ["musl_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # macOS qualification harness; see tools/ci/macos_qualification.sh.
    sh_binary(
        name = "macos_qualification",
        srcs = ["macos_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Windows qualification harness; see tools/ci/windows_qualification.sh.
    sh_binary(
        name = "windows_qualification",
        srcs = ["windows_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Host-matrix harness; see tools/ci/ci_matrix_qualification.sh.
    sh_binary(
        name = "ci_matrix_qualification",
        srcs = ["ci_matrix_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Per-host skip-budget harness; see docs/testing/tools.md.
    sh_binary(
        name = "skip_budget_qualification",
        srcs = ["skip_budget_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Bootstrap portability harness; see tools/ci/bootstrap_portability.sh.
    sh_binary(
        name = "bootstrap_portability",
        srcs = ["bootstrap_portability.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # CI flakiness plus timeout tuning harness; see tools/ci/flakiness_qualification.sh.
    sh_binary(
        name = "flakiness_qualification",
        srcs = ["flakiness_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Runner plus SDK rotation harness (issue #642; See: docs/github-ci.md#runner-plus-sdk-rotation).
    sh_binary(
        name = "runner_rotation_qualification",
        srcs = ["runner_rotation_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # GHCR rebuild plus signing rotation harness (issue #647; See: docs/contributing/devcontainer.md#ghcr-rebuild-plus-signing-rotation).
    sh_binary(
        name = "ghcr_rebuild_rotation_qualification",
        srcs = ["ghcr_rebuild_rotation_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Devcontainer boot manual plus wont-fix harness (issue #648; See: docs/contributing/devcontainer.md#container-boot-manual).
    sh_binary(
        name = "devcontainer_boot_qualification",
        srcs = ["devcontainer_boot_qualification.sh"],
        data = ["//tools/sh:bootstrap",
"//tools/sh:lib"],
        # Bash-only harness is Linux-only (shell contract).
        target_compatible_with = ["@platforms//os:linux"],
    )

    # Env/codegen harness; see tools/ci/env_codegen_qualification.sh.
