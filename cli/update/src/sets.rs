#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SetId {
    Cargo,
    Go,
    Maven,
    Npm,
    NuGet,
}

impl SetId {
    pub const ALL: [SetId; 5] = [
        SetId::Cargo,
        SetId::Go,
        SetId::Maven,
        SetId::Npm,
        SetId::NuGet,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SetId::Cargo => "cargo",
            SetId::Go => "go",
            SetId::Maven => "maven",
            SetId::Npm => "npm",
            SetId::NuGet => "nuget",
        }
    }

    pub fn parse(text: &str) -> Option<SetId> {
        match text {
            "cargo" => Some(SetId::Cargo),
            "go" => Some(SetId::Go),
            "maven" => Some(SetId::Maven),
            "npm" => Some(SetId::Npm),
            "nuget" => Some(SetId::NuGet),
            _ => None,
        }
    }

    pub fn manifests(self) -> &'static [&'static str] {
        match self {
            SetId::Cargo => &["rust/tests/fixtures/hello/Cargo.toml"],
            SetId::Go => &["third_party/go/go.mod"],
            SetId::Maven => &["MODULE.bazel"],
            SetId::Npm => &["package.json"],
            SetId::NuGet => &["third_party/dotnet/paket.dependencies"],
        }
    }

    pub fn locks(self) -> &'static [&'static str] {
        match self {
            SetId::Cargo => &[
                "rust/tests/fixtures/hello/Cargo.lock",
                "cargo-bazel-lock.json",
            ],
            SetId::Go => &["third_party/go/go.mod", "third_party/go/go.sum"],
            SetId::Maven => &["third_party/jvm/maven_install.json"],
            SetId::Npm => &["pnpm-lock.yaml"],
            SetId::NuGet => &["third_party/dotnet/paket.lock", "third_party/dotnet/deps"],
        }
    }

    pub fn updater(self) -> &'static str {
        match self {
            SetId::Cargo => {
                "crate_universe repin (CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello)"
            }
            SetId::Go => {
                "pinned go_deps.from_file module lock (pins track Gazelle; explicit widen via `dx bump` plus the pinned SDK tidy)"
            }
            SetId::Maven => "rules_jvm_external pin (REPIN=1 bazel run @maven//:pin)",
            SetId::Npm => "Bazel-pinned pnpm update (bazel run @pnpm//:pnpm -- update)",
            SetId::NuGet => {
                "paket2bazel regeneration (bazel run @rules_dotnet//tools/paket2bazel -- ...)"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn all_sets_are_distinct_and_parse_round_trip() {
        let mut seen = BTreeSet::new();
        for set in SetId::ALL {
            assert!(seen.insert(set.name()), "duplicate set name");
            assert_eq!(SetId::parse(set.name()), Some(set));
        }
        assert_eq!(seen.len(), 5);
        assert_eq!(SetId::parse("Cargo"), None);
        assert_eq!(SetId::parse("cargo-lock"), None);
        assert_eq!(SetId::parse(""), None);
    }

    #[test]
    fn order_is_alphabetical_and_deterministic() {
        let names: Vec<&str> = SetId::ALL.iter().map(|set| set.name()).collect();
        assert_eq!(names, vec!["cargo", "go", "maven", "npm", "nuget"]);
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn no_set_owns_a_dx_lockfile() {
        for set in SetId::ALL {
            for path in set.manifests().iter().chain(set.locks().iter()) {
                assert!(
                    !path.contains("dx.lock") && !path.contains(".dx-lock"),
                    "{set:?} must not own a dx lockfile: {path}"
                );
            }
        }
    }

    #[test]
    fn every_set_owns_manifests_and_locks() {
        for set in SetId::ALL {
            assert!(!set.manifests().is_empty(), "{set:?} owns a manifest");
            assert!(!set.locks().is_empty(), "{set:?} owns a lock");
        }
        assert_eq!(SetId::Go.manifests(), &["third_party/go/go.mod"]);
        assert_eq!(
            SetId::Go.locks(),
            &["third_party/go/go.mod", "third_party/go/go.sum"]
        );
    }

    #[test]
    fn cargo_npm_maven_nuget_paths_are_pinned() {
        assert_eq!(
            SetId::Cargo.manifests(),
            &["rust/tests/fixtures/hello/Cargo.toml"]
        );
        assert_eq!(
            SetId::Cargo.locks(),
            &[
                "rust/tests/fixtures/hello/Cargo.lock",
                "cargo-bazel-lock.json"
            ]
        );
        assert_eq!(SetId::Npm.manifests(), &["package.json"]);
        assert_eq!(SetId::Npm.locks(), &["pnpm-lock.yaml"]);
        assert_eq!(
            SetId::Maven.locks(),
            &["third_party/jvm/maven_install.json"]
        );
        assert!(SetId::NuGet
            .locks()
            .contains(&"third_party/dotnet/paket.lock"));
    }
}
