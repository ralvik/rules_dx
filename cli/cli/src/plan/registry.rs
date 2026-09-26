use super::{CLIPPY_DIAGNOSTICS_FLAG, RUSTC_DIAGNOSTICS_FLAG};
use crate::args::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    pub command: Command,
    pub capability: &'static str,
    pub aspects: &'static [&'static str],
    pub reports: &'static [&'static str],
    pub settings: &'static [&'static str],
}

pub fn spec(command: Command) -> CommandSpec {
    match command {
        Command::Lint => CommandSpec {
            command,
            capability: "lint",
            aspects: &[
                "//quality:real_aspects.bzl%real_lint_aspect",
                "//quality:real_aspects.bzl%real_js_lint_aspect",
                "//quality:real_aspects.bzl%real_python_lint_aspect",
                "//quality:real_aspects.bzl%real_jvm_lint_aspect",
                "//quality:real_aspects.bzl%real_rust_lint_aspect",
            ],
            reports: &["sarif"],
            settings: &[CLIPPY_DIAGNOSTICS_FLAG],
        },
        Command::Typecheck => CommandSpec {
            command,
            capability: "typecheck",
            aspects: &[
                "//quality:real_aspects.bzl%real_typecheck_aspect",
                "//quality:real_aspects.bzl%real_rust_typecheck_aspect",
            ],
            reports: &["sarif"],
            settings: &[RUSTC_DIAGNOSTICS_FLAG],
        },
        Command::Format => CommandSpec {
            command,
            capability: "format",
            aspects: &[
                "//quality:real_aspects.bzl%real_format_aspect",
                "//quality:real_aspects.bzl%real_js_format_aspect",
                "//quality:real_aspects.bzl%real_jvm_format_aspect",
                "//quality:real_aspects.bzl%real_rust_format_aspect",
            ],
            reports: &[],
            settings: &[],
        },
        Command::Generate => CommandSpec {
            command,
            capability: "generate",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        Command::Build => CommandSpec {
            command,
            capability: "build",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        Command::Test => CommandSpec {
            command,
            capability: "test",
            aspects: &[],
            reports: &["junit"],
            settings: &[],
        },
        Command::Coverage => CommandSpec {
            command,
            capability: "coverage",
            aspects: &[],
            reports: &["lcov"],
            settings: &[],
        },
        Command::Run => CommandSpec {
            command,
            capability: "run",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        Command::Deploy => CommandSpec {
            command,
            capability: "deploy",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Sequential umbrellas (WP4) never build Bazel
        // invocations of their own; phases reuse their registries
        // verbatim. The umbrella routes SARIF requests to the
        // SARIF-capable phases (lint, typecheck) and merges runs.
        Command::Check => CommandSpec {
            command,
            capability: "check",
            aspects: &[],
            reports: &["sarif"],
            settings: &[],
        },
        Command::Fix => CommandSpec {
            command,
            capability: "fix",
            aspects: &[],
            reports: &["sarif"],
            settings: &[],
        },
        // Explicit managed-state cleanup: no Bazel
        // invocation of its own for the prune itself (filesystem
        // inventory plus the shared commit lock in `dx_clean`); the
        // optional `bazel clean` forward is planned at execution.
        Command::Clean => CommandSpec {
            command,
            capability: "clean",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Managed environment/codegen/setup selections: one
        // Bazel collection request behind a canonical selection plus
        // generation commit, never the quality aspect pipeline and no
        // standard reports. The capability names the selecting command.
        Command::Codegen | Command::Env | Command::Setup => CommandSpec {
            command,
            capability: command.name(),
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Delivered adoption/inspect surfaces: local helpers or
        // thin query forwarding, never the quality aspect pipeline.
        // `new` scaffolds a minimal qualified project per language;
        // `upgrade` composes pin plus migrate plus setup with a recovery
        Command::Init
        | Command::New
        | Command::Upgrade
        | Command::Hooks
        | Command::Status
        | Command::Version
        | Command::Watch
        | Command::Owners
        | Command::Deps
        | Command::Why
        | Command::Completion => CommandSpec {
            command,
            capability: "adoption",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Security/license surfaces: dependency-set selectors plan
        // through the `dx_audit` library, never the quality aspect
        // pipeline. Security exports SARIF findings and license exports
        // SPDX 2.3 JSON through the shared report contract; both formats
        // stay accepted on either command and the executor selects the
        // family payload. Update reports per-set
        // through live output (text plus `notice`/`error` in JSON, issue
        // with no `--report` standard report.
        Command::Security | Command::License => CommandSpec {
            command,
            capability: "audit",
            aspects: &[],
            reports: &["sarif", "spdx"],
            settings: &[],
        },
        Command::Update => CommandSpec {
            command,
            capability: "update",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Explicit widen-one-requirement: one declared
        // requirement to a new version through `dx_bump`, never the
        // quality aspect pipeline and no standard reports.
        Command::Bump => CommandSpec {
            command,
            capability: "bump",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Upgrade migration: `--from`/`--to`
        // versions through `dx_adopt::plan_migrate`, never the quality
        // aspect pipeline and no standard reports. Live execution
        // fails closed until the first manifest lands.
        Command::Migrate => CommandSpec {
            command,
            capability: "migrate",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Docs site build over the Bazel-cached extract to aggregate to
        // render chain: no aspects, no standard reports; planned at
        // execution as `bazel build` over the resolved docs targets.
        Command::Docs => CommandSpec {
            command,
            capability: "docs",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
        // Raw launcher passthrough (WP4 helper surface): no
        // aspects, no reports, no scope resolution; planned at
        // execution as launcher plus forwarded arguments.
        Command::Bazel => CommandSpec {
            command,
            capability: "bazel",
            aspects: &[],
            reports: &[],
            settings: &[],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::WorkflowVerb;

    #[test]
    fn registry_entries_match_command_contract() {
        let lint = spec(Command::Lint);
        assert_eq!(lint.capability, "lint");
        assert_eq!(
            lint.aspects,
            &[
                "//quality:real_aspects.bzl%real_lint_aspect",
                "//quality:real_aspects.bzl%real_js_lint_aspect",
                "//quality:real_aspects.bzl%real_python_lint_aspect",
                "//quality:real_aspects.bzl%real_jvm_lint_aspect",
                "//quality:real_aspects.bzl%real_rust_lint_aspect",
            ]
        );
        assert_eq!(lint.reports, &["sarif"]);
        assert_eq!(lint.settings, &[CLIPPY_DIAGNOSTICS_FLAG]);
        let typecheck = spec(Command::Typecheck);
        assert_eq!(typecheck.capability, "typecheck");
        assert_eq!(
            typecheck.aspects,
            &[
                "//quality:real_aspects.bzl%real_typecheck_aspect",
                "//quality:real_aspects.bzl%real_rust_typecheck_aspect",
            ]
        );
        assert_eq!(typecheck.reports, &["sarif"]);
        assert_eq!(typecheck.settings, &[RUSTC_DIAGNOSTICS_FLAG]);
        let format = spec(Command::Format);
        assert_eq!(format.capability, "format");
        assert_eq!(
            format.aspects,
            &[
                "//quality:real_aspects.bzl%real_format_aspect",
                "//quality:real_aspects.bzl%real_js_format_aspect",
                "//quality:real_aspects.bzl%real_jvm_format_aspect",
                "//quality:real_aspects.bzl%real_rust_format_aspect",
            ]
        );
        assert!(format.reports.is_empty());
        let build = spec(Command::Build);
        assert!(build.aspects.is_empty());
        assert!(build.reports.is_empty());
        let test = spec(Command::Test);
        assert_eq!(test.reports, &["junit"]);
        let coverage = spec(Command::Coverage);
        assert_eq!(coverage.reports, &["lcov"]);
        let run = spec(Command::Run);
        assert_eq!(run.capability, "run");
        assert!(run.aspects.is_empty());
        assert!(run.reports.is_empty());
        let generate = spec(Command::Generate);
        assert_eq!(generate.capability, "generate");
        assert!(generate.aspects.is_empty());
        assert!(generate.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Generate), None);
        assert_eq!(WorkflowVerb::of(Command::Run), Some(WorkflowVerb::Run));
        assert_eq!(WorkflowVerb::of(Command::Lint), None);
        assert_eq!(WorkflowVerb::of(Command::Typecheck), None);
        assert_eq!(WorkflowVerb::of(Command::Format), None);
        assert_eq!(WorkflowVerb::of(Command::Check), None);
        assert_eq!(WorkflowVerb::of(Command::Fix), None);
        let clean = spec(Command::Clean);
        assert_eq!(clean.capability, "clean");
        assert!(clean.aspects.is_empty());
        assert!(clean.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Clean), None);
        let bazel = spec(Command::Bazel);
        assert_eq!(bazel.capability, "bazel");
        assert!(bazel.aspects.is_empty());
        assert!(bazel.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Bazel), None);
        for command in [Command::Codegen, Command::Env, Command::Setup] {
            let entry = spec(command);
            assert_eq!(entry.capability, command.name());
            assert!(entry.aspects.is_empty());
            assert!(entry.reports.is_empty());
            assert_eq!(WorkflowVerb::of(command), None);
            assert!(command.is_managed());
        }
        for command in [Command::Security, Command::License] {
            let entry = spec(command);
            assert_eq!(entry.capability, "audit");
            assert!(entry.aspects.is_empty());
            assert_eq!(entry.reports, &["sarif", "spdx"]);
            assert_eq!(WorkflowVerb::of(command), None);
            assert!(command.is_audit_update());
        }
        let update = spec(Command::Update);
        assert_eq!(update.capability, "update");
        assert!(update.aspects.is_empty());
        assert!(update.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Update), None);
        assert!(Command::Update.is_audit_update());
        let bump = spec(Command::Bump);
        assert_eq!(bump.capability, "bump");
        assert!(bump.aspects.is_empty());
        assert!(bump.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Bump), None);
        assert!(Command::Bump.is_audit_update());
        let migrate = spec(Command::Migrate);
        assert_eq!(migrate.capability, "migrate");
        assert!(migrate.aspects.is_empty());
        assert!(migrate.reports.is_empty());
        assert_eq!(WorkflowVerb::of(Command::Migrate), None);
        assert!(!Command::Migrate.is_audit_update());
        assert_eq!(WorkflowVerb::Run.name(), "run");
        assert!(!WorkflowVerb::Run.collects_reports());
    }
}
