#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const REPOSITORY_PATTERN: &str = "//...";

pub const PATTERN_FILE_FLAG: &str = "--target_pattern_file";

pub const WARM_WEIGHT: u64 = 2;

pub const FROZEN_STRATEGY: RootStrategy = RootStrategy::RecursivePattern;

pub fn frozen_strategy() -> RootStrategy {
    FROZEN_STRATEGY
}

pub const FROZEN_EVIDENCE: [(RootStrategy, bool, u64, u64); 4] = [
    (RootStrategy::RecursivePattern, true, 8457, 372),
    (RootStrategy::QueryPatternFile, true, 8983, 350),
    (RootStrategy::MonolithicAggregate, false, 2300, 329),
    (RootStrategy::PackageShards, false, 2000, 300),
];

pub const INCREMENTALITY_EVIDENCE: [(BenchmarkDimension, u64, u64, u64); 3] = [
    (BenchmarkDimension::SourceEdit, 446, 471, 2),
    (BenchmarkDimension::BuildEdit, 442, 469, 0),
    (BenchmarkDimension::TargetAddRemove, 531, 542, 0),
];

pub const PLAN_MATERIALIZED_FILES: u64 = 3;

pub const PLAN_MATERIALIZED_BYTES: u64 = 238;

pub const PLAN_GROUP_WARM_MS: u64 = 413;

pub const DEFAULT_OUTPUTS_WARM_MS: u64 = 502;

pub const SERVER_PEAK_RSS_KB: u64 = 2628812;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RootStrategy {
    RecursivePattern,
    QueryPatternFile,
    MonolithicAggregate,
    PackageShards,
}

impl RootStrategy {
    pub const ALL: [RootStrategy; 4] = [
        RootStrategy::RecursivePattern,
        RootStrategy::QueryPatternFile,
        RootStrategy::MonolithicAggregate,
        RootStrategy::PackageShards,
    ];

    pub fn baseline() -> RootStrategy {
        RootStrategy::RecursivePattern
    }

    pub fn name(&self) -> &'static str {
        match self {
            RootStrategy::RecursivePattern => "recursive-pattern",
            RootStrategy::QueryPatternFile => "query-pattern-file",
            RootStrategy::MonolithicAggregate => "monolithic-aggregate",
            RootStrategy::PackageShards => "package-shards",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRootPlan {
    pub strategy: RootStrategy,
    pub roots: Vec<String>,
    pub pattern_file: Option<PathBuf>,
}

impl RepositoryRootPlan {
    pub fn baseline() -> RepositoryRootPlan {
        RepositoryRootPlan {
            strategy: RootStrategy::RecursivePattern,
            roots: vec![REPOSITORY_PATTERN.to_owned()],
            pattern_file: None,
        }
    }

    pub fn query_pattern_file(path: &Path) -> RepositoryRootPlan {
        RepositoryRootPlan {
            strategy: RootStrategy::QueryPatternFile,
            roots: Vec::new(),
            pattern_file: Some(path.to_owned()),
        }
    }

    pub fn monolithic_aggregate(label: &str) -> RepositoryRootPlan {
        RepositoryRootPlan {
            strategy: RootStrategy::MonolithicAggregate,
            roots: vec![label.to_owned()],
            pattern_file: None,
        }
    }

    pub fn package_shards(labels: &[String]) -> RepositoryRootPlan {
        RepositoryRootPlan {
            strategy: RootStrategy::PackageShards,
            roots: labels.to_vec(),
            pattern_file: None,
        }
    }

    pub fn pattern_file_arg(&self) -> Option<String> {
        self.pattern_file
            .as_ref()
            .map(|path| format!("{}={}", PATTERN_FILE_FLAG, path.display()))
    }
}

pub fn repository_plan() -> RepositoryRootPlan {
    debug_assert_eq!(frozen_strategy(), RootStrategy::baseline());
    RepositoryRootPlan::baseline()
}

pub fn invocation_targets(plan: &RepositoryRootPlan, canonical: &str) -> Vec<String> {
    invocation_targets_union(plan, &[canonical])
}

pub fn invocation_targets_union(plan: &RepositoryRootPlan, canonicals: &[&str]) -> Vec<String> {
    if plan.pattern_file.is_some() {
        return Vec::new();
    }
    if plan.strategy == RootStrategy::RecursivePattern
        && plan.roots == vec![REPOSITORY_PATTERN.to_owned()]
    {
        return canonicals
            .iter()
            .map(|canonical| (*canonical).to_owned())
            .collect();
    }
    plan.roots.clone()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactScopeError {
    MultipleTargets { count: usize },
    TargetPattern { value: String },
    NotTargetLabel { value: String },
}

impl std::fmt::Display for ExactScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExactScopeError::MultipleTargets { count } => {
                write!(f, "expected at most one target, found {count}")
            }
            ExactScopeError::TargetPattern { value } => {
                write!(f, "invalid target {value:?}: patterns never select scope")
            }
            ExactScopeError::NotTargetLabel { value } => {
                write!(f, "invalid target {value:?}: want an exact // or @ label")
            }
        }
    }
}

impl std::error::Error for ExactScopeError {}

pub fn resolve_exact_target(targets: &[String]) -> Result<Option<String>, ExactScopeError> {
    match targets {
        [] => Ok(None),
        [single] => {
            if single.contains("...") || single.contains('*') || single.contains('?') {
                Err(ExactScopeError::TargetPattern {
                    value: single.clone(),
                })
            } else if single.starts_with("//") || single.starts_with('@') {
                Ok(Some(single.clone()))
            } else {
                Err(ExactScopeError::NotTargetLabel {
                    value: single.clone(),
                })
            }
        }
        _ => Err(ExactScopeError::MultipleTargets {
            count: targets.len(),
        }),
    }
}

pub fn build_argv(
    plan: &RepositoryRootPlan,
    canonical: &str,
    aspects: &[String],
    output_groups: &[String],
) -> Vec<String> {
    build_argv_union(plan, &[canonical], aspects, output_groups)
}

pub fn build_argv_union(
    plan: &RepositoryRootPlan,
    canonicals: &[&str],
    aspects: &[String],
    output_groups: &[String],
) -> Vec<String> {
    let mut argv = vec!["build".to_owned()];
    argv.extend(invocation_targets_union(plan, canonicals));
    for aspect in aspects {
        argv.push(format!("--aspects={aspect}"));
    }
    for group in output_groups {
        argv.push(format!("--output_groups={group}"));
    }
    if let Some(pattern_arg) = plan.pattern_file_arg() {
        argv.push(pattern_arg);
    }
    argv
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    pub missing: Vec<String>,
    pub extra: Vec<String>,
}

impl CoverageReport {
    pub fn is_covered(&self) -> bool {
        self.missing.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "root candidate drops {missing_len} designated root(s): {missing_list}",
    missing_len = missing.len(),
    missing_list = missing.join(", ")
)]
pub struct CoverageError {
    pub missing: Vec<String>,
}

fn sorted_set(labels: &[String]) -> BTreeSet<String> {
    labels.iter().cloned().collect()
}

pub fn check_semantic_coverage(
    candidate_roots: &[String],
    designated_roots: &[String],
) -> Result<CoverageReport, CoverageError> {
    let candidate = sorted_set(candidate_roots);
    let designated = sorted_set(designated_roots);
    let missing: Vec<String> = designated.difference(&candidate).cloned().collect();
    if !missing.is_empty() {
        return Err(CoverageError { missing });
    }
    let extra: Vec<String> = candidate.difference(&designated).cloned().collect();
    Ok(CoverageReport { missing, extra })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BenchmarkDimension {
    ColdBuild,
    WarmBuild,
    SourceEdit,
    BuildEdit,
    TargetAddRemove,
    Actions,
    MaterializedBytes,
    ProjectionTime,
    RetainedMemory,
}

impl BenchmarkDimension {
    pub const ALL: [BenchmarkDimension; 9] = [
        BenchmarkDimension::ColdBuild,
        BenchmarkDimension::WarmBuild,
        BenchmarkDimension::SourceEdit,
        BenchmarkDimension::BuildEdit,
        BenchmarkDimension::TargetAddRemove,
        BenchmarkDimension::Actions,
        BenchmarkDimension::MaterializedBytes,
        BenchmarkDimension::ProjectionTime,
        BenchmarkDimension::RetainedMemory,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            BenchmarkDimension::ColdBuild => "cold-build",
            BenchmarkDimension::WarmBuild => "warm-build",
            BenchmarkDimension::SourceEdit => "source-edit",
            BenchmarkDimension::BuildEdit => "build-edit",
            BenchmarkDimension::TargetAddRemove => "target-add-remove",
            BenchmarkDimension::Actions => "actions",
            BenchmarkDimension::MaterializedBytes => "materialized-bytes",
            BenchmarkDimension::ProjectionTime => "projection-time",
            BenchmarkDimension::RetainedMemory => "retained-memory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkSample {
    pub strategy: RootStrategy,
    pub equivalent: bool,
    pub cold_ms: u64,
    pub warm_ms: u64,
}

pub fn weighted_score(sample: &BenchmarkSample) -> u128 {
    u128::from(sample.cold_ms)
        .saturating_add(u128::from(sample.warm_ms).saturating_mul(u128::from(WARM_WEIGHT)))
}

pub fn select_strategy(samples: &[BenchmarkSample]) -> RootStrategy {
    let order = |strategy: &RootStrategy| {
        RootStrategy::ALL
            .iter()
            .position(|candidate| candidate == strategy)
            .unwrap_or(usize::MAX)
    };
    samples
        .iter()
        .filter(|sample| sample.equivalent)
        .min_by(|left, right| {
            weighted_score(left)
                .cmp(&weighted_score(right))
                .then_with(|| order(&left.strategy).cmp(&order(&right.strategy)))
        })
        .map(|sample| sample.strategy)
        .unwrap_or_else(RootStrategy::baseline)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(value: &str) -> String {
        value.to_owned()
    }

    fn labels(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| label(value)).collect()
    }

    fn sample(
        strategy: RootStrategy,
        equivalent: bool,
        cold_ms: u64,
        warm_ms: u64,
    ) -> BenchmarkSample {
        BenchmarkSample {
            strategy,
            equivalent,
            cold_ms,
            warm_ms,
        }
    }

    #[test]
    fn baseline_plan_applies_aspects_to_recursive_pattern() {
        let plan = RepositoryRootPlan::baseline();
        assert_eq!(plan.strategy, RootStrategy::RecursivePattern);
        assert_eq!(plan.roots, vec![REPOSITORY_PATTERN.to_owned()]);
        assert_eq!(plan.pattern_file, None);
        assert_eq!(plan.pattern_file_arg(), None);
    }

    #[test]
    fn baseline_constructor_matches_recursive_strategy() {
        assert_eq!(RootStrategy::baseline(), RootStrategy::RecursivePattern);
        assert_eq!(select_strategy(&[]), RootStrategy::RecursivePattern);
    }

    #[test]
    fn strategy_names_are_stable_and_distinct() {
        let names: Vec<&str> = RootStrategy::ALL.iter().map(RootStrategy::name).collect();
        assert_eq!(
            names,
            vec![
                "recursive-pattern",
                "query-pattern-file",
                "monolithic-aggregate",
                "package-shards",
            ]
        );
        let unique: BTreeSet<&str> = names.into_iter().collect();
        assert_eq!(unique.len(), RootStrategy::ALL.len());
    }

    #[test]
    fn all_lists_every_strategy_once() {
        let mut seen = BTreeSet::new();
        for strategy in RootStrategy::ALL {
            assert!(seen.insert(strategy), "duplicate {strategy:?}");
        }
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn query_file_plan_defers_roots_to_flag_file() {
        let plan = RepositoryRootPlan::query_pattern_file(Path::new("/tmp/roots.txt"));
        assert_eq!(plan.strategy, RootStrategy::QueryPatternFile);
        assert!(plan.roots.is_empty());
        assert_eq!(plan.pattern_file, Some(PathBuf::from("/tmp/roots.txt")));
        assert_eq!(
            plan.pattern_file_arg(),
            Some(format!("{PATTERN_FILE_FLAG}=/tmp/roots.txt"))
        );
    }

    #[test]
    fn aggregate_plans_carry_their_labels() {
        let single = RepositoryRootPlan::monolithic_aggregate("//dx:codegen_roots");
        assert_eq!(single.strategy, RootStrategy::MonolithicAggregate);
        assert_eq!(single.roots, vec![label("//dx:codegen_roots")]);
        assert_eq!(single.pattern_file_arg(), None);

        let shards = RepositoryRootPlan::package_shards(&labels(&["//a:roots", "//b:roots"]));
        assert_eq!(shards.strategy, RootStrategy::PackageShards);
        assert_eq!(shards.roots, labels(&["//a:roots", "//b:roots"]));
        assert_eq!(shards.pattern_file_arg(), None);
    }

    #[test]
    fn full_coverage_passes_with_extras_reported() {
        let report = check_semantic_coverage(
            &labels(&["//a:gen", "//b:gen", "//c:tool"]),
            &labels(&["//b:gen", "//a:gen"]),
        )
        .expect("covered");
        assert!(report.is_covered());
        assert!(report.missing.is_empty());
        assert_eq!(report.extra, vec![label("//c:tool")]);
    }

    #[test]
    fn duplicate_roots_are_inert() {
        let report = check_semantic_coverage(
            &labels(&["//a:gen", "//a:gen", "//b:gen"]),
            &labels(&["//a:gen", "//b:gen"]),
        )
        .expect("covered");
        assert!(report.is_covered());
        assert!(report.extra.is_empty());
    }

    #[test]
    fn missing_designated_root_fails_closed() {
        let error =
            check_semantic_coverage(&labels(&["//a:gen"]), &labels(&["//a:gen", "//b:gen"]))
                .unwrap_err();
        assert_eq!(error.missing, vec![label("//b:gen")]);
        assert!(!format!("{error}").is_empty());
    }

    #[test]
    fn empty_index_never_covers_designated_roots() {
        let error = check_semantic_coverage(&[], &labels(&["//a:gen"])).unwrap_err();
        assert_eq!(error.missing, vec![label("//a:gen")]);
    }

    #[test]
    fn empty_designated_set_is_covered() {
        let report = check_semantic_coverage(&labels(&["//a:gen"]), &[]).expect("covered");
        assert!(report.is_covered());
        assert_eq!(report.extra, vec![label("//a:gen")]);
    }

    #[test]
    fn warm_is_weighted_above_cold() {
        let cold_fast = sample(RootStrategy::RecursivePattern, true, 100, 100);
        let warm_fast = sample(RootStrategy::QueryPatternFile, true, 150, 50);
        assert_eq!(weighted_score(&cold_fast), 300);
        assert_eq!(weighted_score(&warm_fast), 250);
        assert_eq!(
            select_strategy(&[cold_fast, warm_fast]),
            RootStrategy::QueryPatternFile
        );
    }

    #[test]
    fn fastest_equivalent_candidate_wins() {
        let samples = vec![
            sample(RootStrategy::RecursivePattern, true, 200, 200),
            sample(RootStrategy::MonolithicAggregate, true, 300, 300),
            sample(RootStrategy::PackageShards, true, 100, 100),
        ];
        assert_eq!(select_strategy(&samples), RootStrategy::PackageShards);
    }

    #[test]
    fn non_equivalent_fastest_never_wins() {
        let samples = vec![
            sample(RootStrategy::RecursivePattern, true, 500, 500),
            sample(RootStrategy::QueryPatternFile, false, 1, 1),
        ];
        assert_eq!(select_strategy(&samples), RootStrategy::RecursivePattern);
    }

    #[test]
    fn ties_and_empty_samples_keep_baseline() {
        let tied = vec![
            sample(RootStrategy::QueryPatternFile, true, 100, 100),
            sample(RootStrategy::RecursivePattern, true, 100, 100),
        ];
        assert_eq!(select_strategy(&tied), RootStrategy::RecursivePattern);
        let none_equivalent = vec![sample(RootStrategy::QueryPatternFile, false, 10, 10)];
        assert_eq!(
            select_strategy(&none_equivalent),
            RootStrategy::RecursivePattern
        );
    }

    #[test]
    fn selection_dimensions_are_named() {
        let names: Vec<&str> = BenchmarkDimension::ALL
            .iter()
            .map(BenchmarkDimension::name)
            .collect();
        assert_eq!(
            names,
            vec![
                "cold-build",
                "warm-build",
                "source-edit",
                "build-edit",
                "target-add-remove",
                "actions",
                "materialized-bytes",
                "projection-time",
                "retained-memory",
            ]
        );
        let unique: BTreeSet<&str> = names.into_iter().collect();
        assert_eq!(unique.len(), BenchmarkDimension::ALL.len());
    }

    #[test]
    fn repository_plan_is_the_baseline() {
        assert_eq!(repository_plan(), RepositoryRootPlan::baseline());
    }

    #[test]
    fn freeze_selects_the_recursive_pattern_baseline() {
        assert_eq!(FROZEN_STRATEGY, RootStrategy::RecursivePattern);
        assert_eq!(frozen_strategy(), RootStrategy::baseline());
        assert_eq!(frozen_strategy().name(), "recursive-pattern");
        assert_eq!(repository_plan(), RepositoryRootPlan::baseline());
    }

    #[test]
    fn frozen_evidence_selects_the_frozen_strategy() {
        let samples: Vec<BenchmarkSample> = FROZEN_EVIDENCE
            .iter()
            .map(|(strategy, equivalent, cold_ms, warm_ms)| BenchmarkSample {
                strategy: *strategy,
                equivalent: *equivalent,
                cold_ms: *cold_ms,
                warm_ms: *warm_ms,
            })
            .collect();
        assert_eq!(samples.len(), RootStrategy::ALL.len());
        for (index, strategy) in RootStrategy::ALL.iter().enumerate() {
            assert_eq!(samples[index].strategy, *strategy);
        }
        assert_eq!(select_strategy(&samples), frozen_strategy());
    }

    #[test]
    fn incrementality_evidence_covers_the_reference_dimensions_once() {
        let dims: Vec<BenchmarkDimension> =
            INCREMENTALITY_EVIDENCE.iter().map(|row| row.0).collect();
        assert_eq!(
            dims,
            vec![
                BenchmarkDimension::SourceEdit,
                BenchmarkDimension::BuildEdit,
                BenchmarkDimension::TargetAddRemove,
            ]
        );
    }

    #[test]
    fn incrementality_control_matches_baseline_within_noise() {
        for (dimension, baseline_ms, queryfile_ms, actions) in INCREMENTALITY_EVIDENCE {
            let slower = baseline_ms.max(queryfile_ms);
            let faster = baseline_ms.min(queryfile_ms);
            assert!(
                slower * 100 <= faster * 120,
                "{dimension:?}: baseline {baseline_ms} ms vs query-file {queryfile_ms} ms diverge past 20%"
            );
            assert!(
                actions <= 2,
                "{dimension:?}: {actions} executed actions exceed the reference maximum"
            );
        }
        assert_eq!(
            INCREMENTALITY_EVIDENCE[0].3, 2,
            "source edits re-execute 2 actions"
        );
    }

    #[test]
    fn plan_materialization_stays_cheaper_than_default_outputs() {
        assert_eq!(PLAN_MATERIALIZED_FILES, 3);
        assert_eq!(PLAN_MATERIALIZED_BYTES, 238);
        assert!(
            PLAN_GROUP_WARM_MS < DEFAULT_OUTPUTS_WARM_MS,
            "plan group {} ms must stay cheaper than default outputs {} ms",
            PLAN_GROUP_WARM_MS,
            DEFAULT_OUTPUTS_WARM_MS
        );
        assert!(SERVER_PEAK_RSS_KB > 0);
    }

    #[test]
    fn baseline_plan_keeps_the_canonical_selection_identity() {
        let plan = repository_plan();
        assert_eq!(
            invocation_targets(&plan, "//dx:codegen"),
            vec![label("//dx:codegen")]
        );
    }

    #[test]
    fn aggregate_plans_pass_their_roots_through() {
        let single = RepositoryRootPlan::monolithic_aggregate("//dx:codegen_roots");
        assert_eq!(
            invocation_targets(&single, "//dx:codegen"),
            vec![label("//dx:codegen_roots")]
        );
        let shards = RepositoryRootPlan::package_shards(&labels(&["//a:roots", "//b:roots"]));
        assert_eq!(
            invocation_targets(&shards, "//dx:codegen"),
            labels(&["//a:roots", "//b:roots"])
        );
    }

    #[test]
    fn query_file_plan_carries_no_command_line_patterns() {
        let plan = RepositoryRootPlan::query_pattern_file(Path::new("/tmp/roots.txt"));
        assert!(invocation_targets(&plan, "//dx:codegen").is_empty());
        let argv = build_argv(
            &plan,
            "//dx:codegen",
            &labels(&["//generation:codegen.bzl%dx_codegen_plan_aspect"]),
            &labels(&["dx_codegen_plans"]),
        );
        assert_eq!(
            argv,
            vec![
                "build".to_owned(),
                "--aspects=//generation:codegen.bzl%dx_codegen_plan_aspect".to_owned(),
                "--output_groups=dx_codegen_plans".to_owned(),
                format!("{PATTERN_FILE_FLAG}=/tmp/roots.txt"),
            ]
        );
    }

    #[test]
    fn build_argv_composes_baseline_invocation() {
        let plan = repository_plan();
        let argv = build_argv(
            &plan,
            "//dx:codegen",
            &labels(&["//generation:codegen.bzl%dx_codegen_plan_aspect"]),
            &labels(&["dx_codegen_plans"]),
        );
        assert_eq!(
            argv,
            vec![
                "build".to_owned(),
                "//dx:codegen".to_owned(),
                "--aspects=//generation:codegen.bzl%dx_codegen_plan_aspect".to_owned(),
                "--output_groups=dx_codegen_plans".to_owned(),
            ]
        );
    }
}
