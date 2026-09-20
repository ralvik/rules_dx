//! Resolver-owned backend operations for `dx update`.
//!
//! Pure argv planning over [`crate::sets::SetId`]: every changed file and
//! invoked operation is attributable to the underlying updater, never to a
//! `dx` lockfile or private resolver (there is none). Backends refresh
//! standard locks or equivalent resolved files within declared requirements
//! ([`crate::semantics`]): exact pins stay constraints, lockfile-only
//! entries may advance, prereleases follow upstream configuration, Git
//! branches may advance while tags/commit pins stay unchanged, and path
//! dependencies are upstream-owned no-ops. Transitive changes remain
//! permitted under upstream resolver semantics; a newer release outside the
//! declared requirements is never a failure.
//!
//! V1 operations (all through approved Bazel integrations, never a CLI
//! filesystem scan):
//! - Cargo (full only): `CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello`
//!   regenerates `cargo-bazel-lock.json` from `Cargo.lock` via
//!   `crate_universe`. Selective `cargo:crate` is unsupported in V1 and
//!   reports `unsupported` rather than silently substituting a full update.
//! - npm (full + selective): `bazel run @pnpm//:pnpm -- update [...]`
//!   through the Bazel-pinned pnpm, refreshing `pnpm-lock.yaml`.
//! - Maven (full only): `REPIN=1 bazel run @maven//:pin` refreshes
//!   `maven_install.json` from `MODULE.bazel` pins (exact pins stay).
//!   Selective `maven:group:artifact` is unsupported in V1.
//! - NuGet (full only): the documented `paket2bazel` regeneration
//!   refreshes `third_party/dotnet/deps` from `paket.dependencies`/
//!   `paket.lock` (exact pin stays). Selective is unsupported in V1.
//! - Go (full only): empty set (no `go.mod` in the main workspace), so a
//!   full update is a no-op success with no launch; selective is
//!   unsupported (unknown package).
//!
//! Summaries never render argv, option values, or environment values; the
//! vectors below are passed directly to the process runner.

use super::selector::SetRequest;
use super::sets::SetId;

/// Planned backend operation for one set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendPlan {
    /// Invoke the updater (`argv[0]` is the binary).
    Run {
        /// Argument vector passed directly to the runner.
        argv: Vec<String>,
        /// Extra environment (parent environment is always inherited).
        env: Vec<(String, String)>,
    },
    /// No-op success (Go full: no `go.mod`, nothing to update).
    Noop,
}

/// Backend planning failure (execution-time per-set failure, exit 1 for
/// the invocation overall via [`crate::outcome`]/[`crate::report`], never
/// a silent success and never a full-update substitution).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BackendError {
    /// Selective update is not supported for this set in V1.
    #[error("unsupported selective update for {set}: {reason}")]
    Unsupported {
        /// Owning set name.
        set: &'static str,
        /// Why selective is unsupported.
        reason: &'static str,
    },
}

/// Plans one set's backend operation.
pub fn plan(set: SetId, request: &SetRequest) -> Result<BackendPlan, BackendError> {
    match (set, request) {
        (SetId::Cargo, SetRequest::Full) => Ok(BackendPlan::Run {
            argv: strings(&["bazel", "build", "//rust/tests/fixtures/hello:hello"]),
            env: vec![("CARGO_BAZEL_REPIN".to_owned(), "1".to_owned())],
        }),
        (SetId::Cargo, SetRequest::Packages(_)) => Err(BackendError::Unsupported {
            set: set.name(),
            reason: "crate_universe repin refreshes the whole Cargo lock; use `dx update cargo` for the set",
        }),
        (SetId::Npm, SetRequest::Full) => Ok(BackendPlan::Run {
            argv: strings(&["bazel", "run", "@pnpm//:pnpm", "--", "update"]),
            env: vec![],
        }),
        (SetId::Npm, SetRequest::Packages(packages)) => Ok(BackendPlan::Run {
            argv: [
                strings(&["bazel", "run", "@pnpm//:pnpm", "--", "update"]),
                packages.clone(),
            ]
            .concat(),
            env: vec![],
        }),
        (SetId::Maven, SetRequest::Full) => Ok(BackendPlan::Run {
            argv: strings(&["bazel", "run", "@maven//:pin"]),
            env: vec![("REPIN".to_owned(), "1".to_owned())],
        }),
        (SetId::Maven, SetRequest::Packages(_)) => Err(BackendError::Unsupported {
            set: set.name(),
            reason:
                "maven pins are exact in MODULE.bazel; use `dx update maven` for the set",
        }),
        (SetId::NuGet, SetRequest::Full) => Ok(BackendPlan::Run {
            argv: strings(&[
                "bazel",
                "run",
                "@rules_dotnet//tools/paket2bazel",
                "--",
                "--dependencies-file",
                "third_party/dotnet/paket.dependencies",
                "--output-folder",
                "third_party/dotnet/deps",
            ]),
            env: vec![],
        }),
        (SetId::NuGet, SetRequest::Packages(_)) => Err(BackendError::Unsupported {
            set: set.name(),
            reason:
                "nuget pins are exact in paket.dependencies; use `dx update nuget` for the set",
        }),
        (SetId::Go, SetRequest::Full) => Ok(BackendPlan::Noop),
        (SetId::Go, SetRequest::Packages(_)) => Err(BackendError::Unsupported {
            set: set.name(),
            reason: "the main workspace has no go.mod; there are no Go packages to select",
        }),
    }
}

fn strings(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_plans_are_pinned_and_hermetic() {
        let cargo = plan(SetId::Cargo, &SetRequest::Full).expect("cargo");
        assert_eq!(
            cargo,
            BackendPlan::Run {
                argv: vec![
                    "bazel".to_owned(),
                    "build".to_owned(),
                    "//rust/tests/fixtures/hello:hello".to_owned(),
                ],
                env: vec![("CARGO_BAZEL_REPIN".to_owned(), "1".to_owned())],
            }
        );
        let npm = plan(SetId::Npm, &SetRequest::Full).expect("npm");
        assert_eq!(
            npm,
            BackendPlan::Run {
                argv: vec![
                    "bazel".to_owned(),
                    "run".to_owned(),
                    "@pnpm//:pnpm".to_owned(),
                    "--".to_owned(),
                    "update".to_owned(),
                ],
                env: vec![],
            }
        );
        let maven = plan(SetId::Maven, &SetRequest::Full).expect("maven");
        assert_eq!(
            maven,
            BackendPlan::Run {
                argv: vec![
                    "bazel".to_owned(),
                    "run".to_owned(),
                    "@maven//:pin".to_owned(),
                ],
                env: vec![("REPIN".to_owned(), "1".to_owned())],
            }
        );
        let nuget = plan(SetId::NuGet, &SetRequest::Full).expect("nuget");
        match nuget {
            BackendPlan::Run { argv, env } => {
                assert_eq!(argv[0], "bazel");
                assert!(argv.contains(&"@rules_dotnet//tools/paket2bazel".to_owned()));
                assert!(argv.contains(&"third_party/dotnet/paket.dependencies".to_owned()));
                assert!(argv.contains(&"third_party/dotnet/deps".to_owned()));
                assert!(env.is_empty());
            }
            BackendPlan::Noop => panic!("nuget runs paket2bazel"),
        }
        assert_eq!(
            plan(SetId::Go, &SetRequest::Full).expect("go"),
            BackendPlan::Noop
        );
    }

    #[test]
    fn npm_selective_delegates_packages_to_pnpm() {
        let selective = plan(
            SetId::Npm,
            &SetRequest::Packages(vec!["jest".to_owned(), "react".to_owned()]),
        )
        .expect("npm selective");
        assert_eq!(
            selective,
            BackendPlan::Run {
                argv: vec![
                    "bazel".to_owned(),
                    "run".to_owned(),
                    "@pnpm//:pnpm".to_owned(),
                    "--".to_owned(),
                    "update".to_owned(),
                    "jest".to_owned(),
                    "react".to_owned(),
                ],
                env: vec![],
            }
        );
    }

    #[test]
    fn non_npm_selective_reports_unsupported_never_full() {
        for (set, packages) in [
            (
                SetId::Cargo,
                SetRequest::Packages(vec!["anyhow".to_owned()]),
            ),
            (
                SetId::Maven,
                SetRequest::Packages(vec!["junit:junit".to_owned()]),
            ),
            (
                SetId::NuGet,
                SetRequest::Packages(vec!["FSharp.Core".to_owned()]),
            ),
            (
                SetId::Go,
                SetRequest::Packages(vec!["rules_dx/go/tests/fixtures/hello".to_owned()]),
            ),
        ] {
            let error = plan(set, &packages).expect_err("unsupported");
            assert!(matches!(error, BackendError::Unsupported { .. }), "{set:?}");
            assert!(error.to_string().contains(set.name()));
        }
    }

    #[test]
    fn argv_never_names_a_dx_lockfile() {
        for set in SetId::ALL {
            if let Ok(BackendPlan::Run { argv, .. }) = plan(set, &SetRequest::Full) {
                for arg in argv {
                    assert!(
                        !arg.contains("dx.lock"),
                        "{set:?} argv must not name a dx lockfile: {arg}"
                    );
                }
            }
        }
    }
}
