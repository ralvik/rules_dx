//! Exact selector syntax and target-to-set resolution for `dx update` (issue #19).
//!
//! V1 syntax (`dx update [selector ...]`):
//! - `cargo` | `npm` | `maven` | `nuget` | `go`: a dependency set (full update).
//! - `set:package`: a package within a set through the upstream updater
//!   (e.g. `npm:react`, `cargo:anyhow`, `maven:junit:junit`,
//!   `nuget:FSharp.Core`, `go:rules_dx/go/tests/fixtures/hello`). Identity mappings are
//!   upstream-native, never a private solver.
//! - Bazel labels/patterns (`//...`, `//rust/tests/fixtures/hello:hello`, `//go/...`),
//!   files (`rust/tests/fixtures/hello/Cargo.toml`, `package.json`), and directories
//!   (`go/tests/fixtures/hello`): resolved to owning sets via the prefix table below.
//!   Bare `//...` selects all sets; `MODULE.bazel` selects all sets because
//!   it declares every ecosystem.
//!
//! Target resolution is a pure prefix mapping over the approved set
//! registry, never a CLI filesystem scan. File ownership uses directory
//! prefixes (e.g. `rust/...` is Cargo); Bazel labels use their package
//! path (e.g. `//cli/...` is Cargo). Python (`python/...`,
//! `quality/tools/python/...`) has no owning set in V1 and fails closed as
//! usage error; `docs` (except `docs/ir`), `libs`, and other non-dependency
//! paths likewise have none. Unknown bare words (e.g. legacy `crates`)
//! fail closed rather than guessing.
//!
//! Resolution combines selectors deterministically: set/target selectors
//! request full updates for their sets; package selectors request package
//! updates for theirs. When a set is both fully selected and package
//! selected, the full update wins (it includes the packages). Package-only
//! sets run selective updates where the backend supports them (V1: npm
//! only); other backends report `unsupported` at execution time rather than
//! silently substituting a full update.

use std::collections::{BTreeMap, BTreeSet};

use super::sets::SetId;

/// One parsed selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Selector {
    /// Full update for a set (e.g. `cargo`).
    Set(SetId),
    /// Package update within a set (e.g. `npm:react`).
    Package(SetId, String),
    /// Bazel label/pattern/file/dir to resolve to owning sets.
    Target(String),
}

/// Selector usage errors (exit 2, never a partial update).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SelectorError {
    /// Empty selector.
    #[error("empty selector")]
    Empty,
    /// Unknown bare word (not a set, package, or target shape).
    #[error("unknown update selector {selector:?}; expected cargo|npm|maven|nuget|go, set:package, or a label/path")]
    UnknownSelector {
        /// Offending spelling.
        selector: String,
    },
    /// Invalid package identity for its set.
    #[error("invalid package {package:?} for set {set}: {reason}")]
    InvalidPackage {
        /// Owning set name.
        set: &'static str,
        /// Offending package spelling.
        package: String,
        /// Why it is invalid.
        reason: &'static str,
    },
    /// Target has no owning dependency set in V1.
    #[error("no owning dependency set for {target:?} (python and non-dependency paths are out of V1 update scope)")]
    NoOwningSet {
        /// Offending target spelling.
        target: String,
    },
}

/// Requested update per set after resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SetRequest {
    /// Full update (set/target selected, or bare run).
    Full,
    /// Package-only update (selective; V1 executes for npm, reports
    /// unsupported for other sets rather than silently widening).
    Packages(Vec<String>),
}

/// Parses one selector spelling.
pub fn parse_selector(text: &str) -> Result<Selector, SelectorError> {
    if text.is_empty() {
        return Err(SelectorError::Empty);
    }
    // Package form `set:package` wins over target shapes, except Bazel
    // labels (`//...`, `//foo:bar`) which start with `/` or `@` and never
    // match `set:...` with a known set prefix.
    if let Some((head, tail)) = text.split_once(':') {
        if let Some(set) = SetId::parse(head) {
            // `//foo:bar` never reaches here: it starts with `/`, so `head`
            // would be `//foo`, not a known set.
            if tail.is_empty() {
                return Err(SelectorError::InvalidPackage {
                    set: set.name(),
                    package: tail.to_owned(),
                    reason: "package identity is empty",
                });
            }
            validate_package(set, tail)?;
            return Ok(Selector::Package(set, tail.to_owned()));
        }
        // A colon outside `set:...` and outside Bazel labels (which start
        // with `//`/`@`) is not a valid bare word; fail closed below.
        // Fall through to target/unknown handling so `//foo:bar` still
        // resolves as a target.
    }
    if let Some(set) = SetId::parse(text) {
        return Ok(Selector::Set(set));
    }
    if is_target_shape(text) {
        return Ok(Selector::Target(text.to_owned()));
    }
    Err(SelectorError::UnknownSelector {
        selector: text.to_owned(),
    })
}

/// True for Bazel labels/patterns and file/dir paths.
fn is_target_shape(text: &str) -> bool {
    text.starts_with("//")
        || text.starts_with('@')
        || text.contains('/')
        || text.contains('.')
        || text == "..."
}

/// Validates an ecosystem package identity (upstream-native, no versions).
fn validate_package(set: SetId, package: &str) -> Result<(), SelectorError> {
    let invalid = |reason: &'static str| SelectorError::InvalidPackage {
        set: set.name(),
        package: package.to_owned(),
        reason,
    };
    match set {
        SetId::Cargo => {
            if package.is_empty()
                || !package
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(invalid("cargo crate names use [A-Za-z0-9_-] only"));
            }
            Ok(())
        }
        SetId::Npm => {
            if package.is_empty() || package.contains(':') || package.contains(' ') {
                return Err(invalid("npm package names never contain ':' or spaces"));
            }
            if let Some(rest) = package.strip_prefix('@') {
                let (scope, slash, name) = match rest.find('/') {
                    Some(idx) => (&rest[..idx], true, &rest[idx + 1..]),
                    None => ("", false, ""),
                };
                let _ = slash;
                if scope.is_empty() || name.is_empty() || !slash {
                    return Err(invalid("scoped npm names are @scope/name"));
                }
                if !scope
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                    || !name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                {
                    return Err(invalid("npm scope/name use [A-Za-z0-9_.-] only"));
                }
                Ok(())
            } else {
                if package.contains('/') {
                    return Err(invalid("unscoped npm names never contain '/'"));
                }
                if !package
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                {
                    return Err(invalid("npm names use [A-Za-z0-9_.-] only"));
                }
                Ok(())
            }
        }
        SetId::Maven => {
            let (group, artifact) = match package.split_once(':') {
                Some((g, a)) => (g, a),
                None => {
                    return Err(invalid("maven identities are group:artifact"));
                }
            };
            if group.is_empty()
                || artifact.is_empty()
                || artifact.contains(':')
                || group.contains(' ')
                || artifact.contains(' ')
            {
                return Err(invalid("maven identities are group:artifact"));
            }
            Ok(())
        }
        SetId::NuGet => {
            if package.is_empty()
                || !package
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            {
                return Err(invalid("nuget ids use [A-Za-z0-9_.-] only"));
            }
            Ok(())
        }
        SetId::Go => {
            if package.is_empty()
                || package.contains(':')
                || package.contains(' ')
                || package.starts_with('/')
                || package.ends_with('/')
                || package.contains("//")
            {
                return Err(invalid("go module paths never contain ':' or spaces"));
            }
            if !package.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | '~' | '+')
            }) {
                return Err(invalid("go module paths use [A-Za-z0-9/_.-~+] only"));
            }
            Ok(())
        }
    }
}

/// Owning sets for one target selector (pure prefix mapping).
pub fn owning_sets(target: &str) -> Vec<SetId> {
    // Repository-wide and module-declaring selectors cover all sets.
    if target == "//..." {
        return SetId::ALL.to_vec();
    }
    if target.contains("MODULE.bazel") {
        return SetId::ALL.to_vec();
    }
    let package = package_path(target);
    // Root package/file handling: `//:target` (empty package) plus root
    // filenames like `package.json` (file paths with no `/` and no label
    // prefix) decide by filename spelling, since the root owns several
    // families. Bazel labels without a slash (e.g. `//quality`) still use
    // prefix matching below.
    if package.is_empty()
        || (!target.starts_with("//") && !target.starts_with('@') && !target.contains('/'))
    {
        return root_owning_sets(target);
    }
    // Longest-prefix first so `quality/tools/javascript` beats `quality`.
    if has_prefix(&package, "quality/tools/javascript") {
        return vec![SetId::Npm];
    }
    if has_prefix(&package, "quality/tools/python") {
        return vec![];
    }
    if has_prefix(&package, "examples/adopt-rust") {
        return vec![SetId::Cargo];
    }
    if has_prefix(&package, "examples/adopt-js-ts") {
        return vec![SetId::Npm];
    }
    if has_prefix(&package, "examples/adopt-java") || has_prefix(&package, "examples/adopt-kotlin")
    {
        return vec![SetId::Maven];
    }
    if has_prefix(&package, "examples/adopt-go") {
        return vec![SetId::Go];
    }
    if has_prefix(&package, "examples/adopt-polyglot") {
        return vec![SetId::Cargo, SetId::Npm];
    }
    if has_prefix(&package, "docs/ir") {
        return vec![SetId::Cargo];
    }
    for prefix in ["rust", "cli", "generation", "env", "quality"] {
        if has_prefix(&package, prefix) {
            return vec![SetId::Cargo];
        }
    }
    for prefix in ["javascript", "typescript", "astro", "svelte", "vue", "mdx"] {
        if has_prefix(&package, prefix) {
            return vec![SetId::Npm];
        }
    }
    for prefix in ["java", "kotlin", "scala", "third_party/jvm"] {
        if has_prefix(&package, prefix) {
            return vec![SetId::Maven];
        }
    }
    for prefix in ["csharp", "fsharp", "third_party/dotnet", "paket-files"] {
        if has_prefix(&package, prefix) {
            return vec![SetId::NuGet];
        }
    }
    if has_prefix(&package, "go") {
        return vec![SetId::Go];
    }
    vec![]
}

/// Package path for prefix matching: Bazel labels use their package,
/// files/dirs use their normalized path.
fn package_path(target: &str) -> String {
    if let Some(rest) = target.strip_prefix("//") {
        // `//:target` (root package) carries no package path.
        if let Some(colon) = rest.find(':') {
            return rest[..colon].to_owned();
        }
        // `//foo/...` and `//foo/bar/...` strip the recursive suffix;
        // `//foo` stays `foo`.
        if let Some(prefix) = rest.strip_suffix("/...") {
            return prefix.to_owned();
        }
        return rest.to_owned();
    }
    if let Some(rest) = target.strip_prefix('@') {
        let _ = rest;
        return String::new();
    }
    // File/dir path: strip leading `./`, trailing `/`, and a trailing
    // `/...` pattern suffix when present.
    let mut path = target.to_owned();
    while let Some(rest) = path.strip_prefix("./") {
        path = rest.to_owned();
    }
    while path.ends_with('/') && path.len() > 1 {
        path.pop();
    }
    if let Some(prefix) = path.strip_suffix("/...") {
        return prefix.to_owned();
    }
    // For files, match on the full path so `rust/tests/fixtures/hello/Cargo.toml`
    // matches `rust` via prefix below.
    path
}

/// Root (`""` package) owning sets by filename/target spelling.
fn root_owning_sets(target: &str) -> Vec<SetId> {
    let lower = target.to_ascii_lowercase();
    if lower.contains("package.json")
        || lower.contains("pnpm-lock.yaml")
        || lower.contains("pnpm-workspace.yaml")
        || lower.contains(".npmrc")
        || lower.contains("node_modules")
        || lower.contains("package_json")
    {
        return vec![SetId::Npm];
    }
    if lower.contains("cargo-bazel-lock.json")
        || lower.contains("cargo.toml")
        || lower.contains("cargo.lock")
    {
        return vec![SetId::Cargo];
    }
    vec![]
}

fn has_prefix(package: &str, prefix: &str) -> bool {
    package == prefix || package.starts_with(&format!("{prefix}/"))
}

/// Resolves selectors to per-set requests (deterministic, sorted).
///
/// Empty input means every supported set (`Full`). Otherwise set/target
/// selectors request `Full` for their sets; package selectors request
/// `Packages` unless their set is also fully selected (full wins).
pub fn resolve(selectors: &[String]) -> Result<BTreeMap<SetId, SetRequest>, SelectorError> {
    if selectors.is_empty() {
        return Ok(SetId::ALL
            .iter()
            .map(|set| (*set, SetRequest::Full))
            .collect());
    }
    let mut full: BTreeSet<SetId> = BTreeSet::new();
    let mut packages: BTreeMap<SetId, Vec<String>> = BTreeMap::new();
    for raw in selectors {
        match parse_selector(raw)? {
            Selector::Set(set) => {
                full.insert(set);
            }
            Selector::Package(set, package) => {
                packages.entry(set).or_default().push(package);
            }
            Selector::Target(target) => {
                let owners = owning_sets(&target);
                if owners.is_empty() {
                    return Err(SelectorError::NoOwningSet { target });
                }
                for set in owners {
                    full.insert(set);
                }
            }
        }
    }
    let mut resolved = BTreeMap::new();
    for set in full {
        resolved.insert(set, SetRequest::Full);
    }
    for (set, mut names) in packages {
        if resolved.contains_key(&set) {
            continue;
        }
        names.sort();
        names.dedup();
        resolved.insert(set, SetRequest::Packages(names));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn parses_sets_packages_and_targets() {
        assert_eq!(parse_selector("cargo"), Ok(Selector::Set(SetId::Cargo)));
        assert_eq!(
            parse_selector("npm:react"),
            Ok(Selector::Package(SetId::Npm, "react".to_owned()))
        );
        assert_eq!(
            parse_selector("maven:junit:junit"),
            Ok(Selector::Package(SetId::Maven, "junit:junit".to_owned()))
        );
        assert_eq!(
            parse_selector("//rust/tests/fixtures/hello:hello"),
            Ok(Selector::Target(
                "//rust/tests/fixtures/hello:hello".to_owned()
            ))
        );
        assert_eq!(
            parse_selector("rust/tests/fixtures/hello/Cargo.toml"),
            Ok(Selector::Target(
                "rust/tests/fixtures/hello/Cargo.toml".to_owned()
            ))
        );
    }

    #[test]
    fn scoped_npm_and_nuget_identities_validate() {
        assert!(parse_selector("npm:@astrojs/compiler").is_ok());
        assert!(parse_selector("nuget:FSharp.Core").is_ok());
        assert!(parse_selector("cargo:anyhow").is_ok());
        assert!(parse_selector("go:rules_dx/go/tests/fixtures/hello").is_ok());
    }

    #[test]
    fn invalid_identities_fail_closed() {
        assert!(matches!(
            parse_selector("cargo:"),
            Err(SelectorError::InvalidPackage { .. })
        ));
        assert!(matches!(
            parse_selector("cargo:bad name"),
            Err(SelectorError::InvalidPackage { .. })
        ));
        assert!(matches!(
            parse_selector("npm:"),
            Err(SelectorError::InvalidPackage { .. })
        ));
        assert!(matches!(
            parse_selector("maven:junit"),
            Err(SelectorError::InvalidPackage { .. })
        ));
        assert!(matches!(
            parse_selector("maven::artifact"),
            Err(SelectorError::InvalidPackage { .. })
        ));
        assert!(matches!(
            parse_selector("nuget:"),
            Err(SelectorError::InvalidPackage { .. })
        ));
        assert!(matches!(
            parse_selector("go:"),
            Err(SelectorError::InvalidPackage { .. })
        ));
        assert!(matches!(parse_selector(""), Err(SelectorError::Empty)));
        assert!(matches!(
            parse_selector("crates"),
            Err(SelectorError::UnknownSelector { .. })
        ));
        assert!(matches!(
            parse_selector("cargo-lock"),
            Err(SelectorError::UnknownSelector { .. })
        ));
    }

    #[test]
    fn bazel_labels_are_targets_not_packages() {
        // `//foo:bar` contains a colon but starts with `//`, so it must
        // never parse as `set:package`.
        assert!(matches!(
            parse_selector("//foo:bar"),
            Ok(Selector::Target(_))
        ));
        assert!(matches!(parse_selector("//..."), Ok(Selector::Target(_))));
    }

    #[test]
    fn owning_sets_cover_the_five_families() {
        assert_eq!(owning_sets("//..."), SetId::ALL.to_vec());
        assert_eq!(owning_sets("MODULE.bazel"), SetId::ALL.to_vec());
        assert_eq!(
            owning_sets("//rust/tests/fixtures/hello:hello"),
            vec![SetId::Cargo]
        );
        assert_eq!(
            owning_sets("rust/tests/fixtures/hello/Cargo.toml"),
            vec![SetId::Cargo]
        );
        assert_eq!(owning_sets("cli/cli/src/exec.rs"), vec![SetId::Cargo]);
        assert_eq!(
            owning_sets("quality/tools/javascript/package.json"),
            vec![SetId::Npm]
        );
        assert_eq!(
            owning_sets("//javascript/tests/fixtures/hello:hello"),
            vec![SetId::Npm]
        );
        assert_eq!(owning_sets("package.json"), vec![SetId::Npm]);
        assert_eq!(
            owning_sets("//third_party/jvm:maven_install"),
            vec![SetId::Maven]
        );
        assert_eq!(
            owning_sets("//csharp/tests/fixtures/hello:hello"),
            vec![SetId::NuGet]
        );
        assert_eq!(
            owning_sets("//go/tests/fixtures/hello:hello"),
            vec![SetId::Go]
        );
        assert_eq!(owning_sets("go/tests/fixtures/hello"), vec![SetId::Go]);
        assert!(owning_sets("python/tests/fixtures/hello/hello.py").is_empty());
        assert!(owning_sets("quality/tools/python/pyproject.toml").is_empty());
        assert!(owning_sets("docs/cli/README.md").is_empty());
        assert!(owning_sets("libs/starlark/defs.bzl").is_empty());
    }

    #[test]
    fn bare_resolves_to_all_sets_full() {
        let resolved = resolve(&[]).expect("bare");
        assert_eq!(resolved.len(), 5);
        for set in SetId::ALL {
            assert_eq!(resolved.get(&set), Some(&SetRequest::Full));
        }
    }

    #[test]
    fn full_wins_over_packages_for_the_same_set() {
        let resolved = resolve(&strings(&["npm", "npm:react"])).expect("full wins");
        assert_eq!(resolved.get(&SetId::Npm), Some(&SetRequest::Full));
    }

    #[test]
    fn package_only_sets_stay_selective_and_sorted() {
        let resolved = resolve(&strings(&["npm:react", "npm:jest"])).expect("packages");
        assert_eq!(
            resolved.get(&SetId::Npm),
            Some(&SetRequest::Packages(vec![
                "jest".to_owned(),
                "react".to_owned()
            ]))
        );
    }

    #[test]
    fn targets_expand_to_owning_sets_full() {
        let resolved = resolve(&strings(&["//go/tests/fixtures/hello:hello"])).expect("target");
        assert_eq!(resolved.get(&SetId::Go), Some(&SetRequest::Full));
        let resolved = resolve(&strings(&["//..."])).expect("repo");
        assert_eq!(resolved.len(), 5);
    }

    #[test]
    fn unknown_and_unowned_targets_fail_closed() {
        assert!(matches!(
            resolve(&strings(&["crates"])),
            Err(SelectorError::UnknownSelector { .. })
        ));
        assert!(matches!(
            resolve(&strings(&["python/tests/fixtures/hello/hello.py"])),
            Err(SelectorError::NoOwningSet { .. })
        ));
        assert!(matches!(
            resolve(&strings(&["docs/cli/README.md"])),
            Err(SelectorError::NoOwningSet { .. })
        ));
    }
}
