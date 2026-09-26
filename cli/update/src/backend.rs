use super::selector::SetRequest;
use super::sets::SetId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendPlan {
    Run {
        argv: Vec<String>,
        env: Vec<(String, String)>,
    },
    Noop,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BackendError {
    #[error("unsupported selective update for {set}: {reason}")]
    Unsupported {
        set: &'static str,
        reason: &'static str,
    },
    #[error("offline_required: cannot update {set} without network (re-run without --offline once connected)")]
    OfflineRequired { set: &'static str },
}

pub fn plan(set: SetId, request: &SetRequest, offline: bool) -> Result<BackendPlan, BackendError> {
    if offline {
        match (set, request) {
            (SetId::Go, SetRequest::Full) => return Ok(BackendPlan::Noop),
            (SetId::Cargo, SetRequest::Packages(_))
            | (SetId::Maven, SetRequest::Packages(_))
            | (SetId::NuGet, SetRequest::Packages(_))
            | (SetId::Go, SetRequest::Packages(_)) => {}
            _ => {
                let would_run = matches!(
                    (set, request),
                    (SetId::Cargo, SetRequest::Full)
                        | (SetId::Npm, _)
                        | (SetId::Maven, SetRequest::Full)
                        | (SetId::NuGet, SetRequest::Full)
                );
                if would_run {
                    return Err(BackendError::OfflineRequired { set: set.name() });
                }
            }
        }
    }
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
            reason: "go pins track Gazelle for the shared go_deps extension; widen explicitly via `dx bump gomod:<module> <version>`",
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
        let cargo = plan(SetId::Cargo, &SetRequest::Full, false).expect("cargo");
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
        let npm = plan(SetId::Npm, &SetRequest::Full, false).expect("npm");
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
        let maven = plan(SetId::Maven, &SetRequest::Full, false).expect("maven");
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
        let nuget = plan(SetId::NuGet, &SetRequest::Full, false).expect("nuget");
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
            plan(SetId::Go, &SetRequest::Full, false).expect("go"),
            BackendPlan::Noop
        );
    }

    #[test]
    fn npm_selective_delegates_packages_to_pnpm() {
        let selective = plan(
            SetId::Npm,
            &SetRequest::Packages(vec!["jest".to_owned(), "react".to_owned()]),
            false,
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
    fn cargo_selective_reports_unsupported_never_full() {
        let error = plan(
            SetId::Cargo,
            &SetRequest::Packages(vec!["anyhow".to_owned()]),
            false,
        )
        .expect_err("cargo selective is wont-fix");
        assert!(
            matches!(error, BackendError::Unsupported { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("cargo"));
        assert!(error
            .to_string()
            .contains("use `dx update cargo` for the set"));
    }

    #[test]
    fn nuget_selective_reports_unsupported_never_full() {
        let error = plan(
            SetId::NuGet,
            &SetRequest::Packages(vec!["FSharp.Core".to_owned()]),
            false,
        )
        .expect_err("nuget selective is wont-fix");
        assert!(
            matches!(error, BackendError::Unsupported { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("nuget"));
        assert!(error
            .to_string()
            .contains("use `dx update nuget` for the set"));
    }

    #[test]
    fn go_selective_reports_unsupported_never_full() {
        let error = plan(
            SetId::Go,
            &SetRequest::Packages(vec!["github.com/google/go-cmp/cmp".to_owned()]),
            false,
        )
        .expect_err("go selective is wont-fix");
        assert!(
            matches!(error, BackendError::Unsupported { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("go"));
        assert!(error.to_string().contains("dx bump gomod"));
    }

    #[test]
    fn go_full_is_pinned_noop_success() {
        assert_eq!(
            plan(SetId::Go, &SetRequest::Full, false).expect("go full"),
            BackendPlan::Noop
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
            let error = plan(set, &packages, false).expect_err("unsupported");
            assert!(matches!(error, BackendError::Unsupported { .. }), "{set:?}");
            assert!(error.to_string().contains(set.name()));
        }
    }

    #[test]
    fn maven_selective_reports_unsupported_with_set_hint() {
        for artifact in [
            "junit:junit".to_owned(),
            "org.junit.jupiter:junit-jupiter-api".to_owned(),
        ] {
            let error = plan(
                SetId::Maven,
                &SetRequest::Packages(vec![artifact.clone()]),
                false,
            )
            .expect_err("maven selective unsupported");
            let message = error.to_string();
            assert!(
                matches!(error, BackendError::Unsupported { .. }),
                "{artifact}"
            );
            assert!(message.contains("maven"), "{artifact}: {message}");
            assert!(
                message.contains("use `dx update maven` for the set"),
                "{artifact}: {message}"
            );
        }
    }

    #[test]
    fn argv_never_names_a_dx_lockfile() {
        for set in SetId::ALL {
            if let Ok(BackendPlan::Run { argv, .. }) = plan(set, &SetRequest::Full, false) {
                for arg in argv {
                    assert!(
                        !arg.contains("dx.lock"),
                        "{set:?} argv must not name a dx lockfile: {arg}"
                    );
                }
            }
        }
    }

    #[test]
    fn offline_forces_cache_only_except_go_noop() {
        for set in [SetId::Cargo, SetId::Npm, SetId::Maven, SetId::NuGet] {
            let error = plan(set, &SetRequest::Full, true).expect_err("offline needs network");
            assert!(
                matches!(error, BackendError::OfflineRequired { .. }),
                "{set:?}: {error:?}"
            );
            assert!(error.to_string().contains("offline_required"), "{error}");
            assert!(error.to_string().contains(set.name()), "{error}");
        }
        let selective = plan(
            SetId::Npm,
            &SetRequest::Packages(vec!["jest".to_owned()]),
            true,
        )
        .expect_err("npm selective offline needs network");
        assert!(
            matches!(selective, BackendError::OfflineRequired { .. }),
            "{selective:?}"
        );
        assert_eq!(
            plan(SetId::Go, &SetRequest::Full, true).expect("go offline noop"),
            BackendPlan::Noop
        );
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
                SetRequest::Packages(vec!["example.com/mod".to_owned()]),
            ),
        ] {
            let error = plan(set, &packages, true).expect_err("unsupported stays unsupported");
            assert!(
                matches!(error, BackendError::Unsupported { .. }),
                "{set:?}: {error:?}"
            );
        }
    }
}
