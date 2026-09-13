//! Repository-root strategy planning for `dx codegen`, `dx env`, and `dx setup` (M25 WP4,
//! O34).
//!
//! Contract: `docs/environments/codegen.md` (repository root selection and
//! performance model) and `docs/environments/environment.md` (repository and
//! target roots). The correctness baseline applies the plan-collection
//! aspects to `//...`. The candidates are a Bazel-query-produced
//! target-pattern file, one monolithic aggregate, and package-local
//! aggregate shards, every candidate keeping identical effective target,
//! aspect, output-group, and configuration semantics. Among
//! equivalent-semantics candidates the fastest wins on cold and warm with
//! warm weighted above cold; otherwise the `//...` baseline remains.
//!
//! This crate freezes the pure planning layer only: candidate identities,
//! the repository root plans behind the canonical `//dx:codegen` and
//! `//dx:env` selections, the designated-root coverage rule (no candidate
//! may reduce roots based only on an unconfigured query graph, since
//! transitions, toolchains, and `select()` can diverge from it), and the
//! benchmark decision rule with its measured dimensions. Slice 2 adds the
//! pure composition of these plans into the codegen/env/setup Bazel
//! invocations (`repository_plan`, `invocation_targets`, `build_argv`);
//! the query/aggregate Starlark wiring lives in `//dx/roots:roots.bzl`.
//! Slice 3 freezes the measured cold/warm winner (the `//...` baseline;
//! see [`frozen_strategy`] and [`FROZEN_EVIDENCE`]). The remaining
//! benchmark dimensions (source/BUILD edits, target churn, actions,
//! materialized bytes, projection time, retained memory) plus concurrency,
//! interruption, remote materialization, and reuse certification land in
//! later WP4 slices; the effective roots stay on the frozen baseline until
//! then.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Correctness-baseline repository pattern: the plan-collection aspects
/// apply to the whole declared BUILD graph. Handles top-level visibility,
/// test-only targets, platform-incompatible skipping, and repository
/// membership, at the cost of recursive package discovery.
pub const REPOSITORY_PATTERN: &str = "//...";

/// Bazel flag carrying the query-produced repository roots: Bazel reads the
/// labels from the file, so command-line length limits do not apply and no
/// central eager `label_list` dependency node is created.
pub const PATTERN_FILE_FLAG: &str = "--target_pattern_file";

/// Warm weight in the O34 decision score (`cold_ms + WARM_WEIGHT *
/// warm_ms`). PROVISIONAL (O34, flagged for review): warm runs dominate
/// developer-iteration latency on internal paths, so warm counts double;
/// the measured WP4 benchmarks may adjust this weight before the strategy
/// freeze.
pub const WARM_WEIGHT: u64 = 2;

/// Frozen O34 repository-root strategy (M25 WP4 slice 3): the `//...`
/// correctness baseline.
///
/// Measured evidence (2026-09-14, Linux x86_64, Bazel 9.2.0 via Bazelisk
/// v1.29.0, warm persistent server unless noted), codegen plan aspect
/// (`//generation:codegen.bzl%dx_codegen_plan_aspect`) with the
/// `dx_codegen_plans` output group throughout:
/// - `recursive-pattern`: `bazel build //...` — cold 8457 ms (after
///   `bazel shutdown`), warm 335/410 ms, 685 analyzed targets with 764
///   aspect applications.
/// - `query-pattern-file`: pattern file holding `//...` (the
///   `roots_pattern_fixture` shape) via `--target_pattern_file` — cold
///   8983 ms, warm 278/381/391 ms, same 685 analyzed targets: equivalent
///   semantics with a file-indirection cost, never faster than direct
///   `//...` on cold.
/// - `monolithic-aggregate`: `//dx/roots:roots_monolith_fixture`
///   (filegroup over the two empty canonical selections) — warm
///   294/329/346 ms over 1 analyzed target: it drops the 685 designated
///   roots, so [`check_semantic_coverage`] fails it closed and
///   [`select_strategy`] excludes it however fast (same for the
///   placeholder canonical-only invocation: cold 2294 ms, warm
///   207/231/245 ms, 1 target).
/// - `package-shards`: no codegen shard fixtures exist yet; unmeasured
///   and therefore excluded as non-equivalent.
///
/// Weighted scores (`cold_ms + WARM_WEIGHT * warm_ms`, warm medians 372
/// vs 350): baseline 8457 + 2*372 = 9201 beats query-file 8983 + 2*350 =
/// 9683 outright; the baseline also wins every tie by [`RootStrategy::ALL`]
/// order. The remaining [`BenchmarkDimension`] rows (source/BUILD edits,
/// target churn, actions, materialized bytes, projection time, retained
/// memory) plus concurrency, interruption, remote materialization, and
/// reuse certification land in later WP4 slices and cannot displace this
/// freeze without new measured evidence plus a freeze change here.
pub const FROZEN_STRATEGY: RootStrategy = RootStrategy::RecursivePattern;

/// Returns the frozen repository-root strategy (see [`FROZEN_STRATEGY`]).
pub fn frozen_strategy() -> RootStrategy {
    FROZEN_STRATEGY
}

/// Headline benchmark samples behind the freeze, in [`RootStrategy::ALL`]
/// order: measured cold/warm wall times in milliseconds with the
/// equivalence flags from the evidence above. [`select_strategy`] over
/// these samples returns [`frozen_strategy`]; the test
/// `frozen_evidence_selects_the_frozen_strategy` pins that implication so
/// the numbers and the freeze cannot drift apart silently.
pub const FROZEN_EVIDENCE: [(RootStrategy, bool, u64, u64); 4] = [
    (RootStrategy::RecursivePattern, true, 8457, 372),
    (RootStrategy::QueryPatternFile, true, 8983, 350),
    (RootStrategy::MonolithicAggregate, false, 2300, 329),
    (RootStrategy::PackageShards, false, 2000, 300),
];

/// Repository-root strategy candidates under O34.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RootStrategy {
    /// Correctness baseline: apply the collecting aspects to `//...`.
    RecursivePattern,
    /// Optimization candidate: pass a Bazel-query-produced label list via
    /// `--target_pattern_file`; Bazel interprets every label and configured
    /// closure, preserving top-level treatment of private, test-only, and
    /// incompatible targets.
    QueryPatternFile,
    /// Benchmark candidate: one monolithic aggregate rule. Must explicitly
    /// propagate aspect providers and expose artifacts through a requested
    /// output group; central-dependency visibility, test-only, platform,
    /// cycle, and fan-out concerns apply.
    MonolithicAggregate,
    /// Benchmark candidate: package-local aggregate shards. Same
    /// propagation and output-group obligations as the monolithic
    /// aggregate, scoped per package.
    PackageShards,
}

impl RootStrategy {
    /// Every candidate, baseline first. Iteration order is the deterministic
    /// tie-break order for benchmark selection: the baseline wins ties.
    pub const ALL: [RootStrategy; 4] = [
        RootStrategy::RecursivePattern,
        RootStrategy::QueryPatternFile,
        RootStrategy::MonolithicAggregate,
        RootStrategy::PackageShards,
    ];

    /// The correctness baseline, and the selected strategy until measured
    /// WP4 evidence freezes a winner.
    pub fn baseline() -> RootStrategy {
        RootStrategy::RecursivePattern
    }

    /// Stable machine-readable identity for reports and benchmark rows.
    pub fn name(&self) -> &'static str {
        match self {
            RootStrategy::RecursivePattern => "recursive-pattern",
            RootStrategy::QueryPatternFile => "query-pattern-file",
            RootStrategy::MonolithicAggregate => "monolithic-aggregate",
            RootStrategy::PackageShards => "package-shards",
        }
    }
}

/// Planned repository roots for one Bazel invocation behind a canonical
/// repository-wide selection (`//dx:codegen`, `//dx:env`, or their union in
/// `dx setup`). Exact-target scopes bypass root selection entirely and never
/// construct this plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRootPlan {
    /// Candidate strategy this plan invokes.
    pub strategy: RootStrategy,
    /// Target patterns or aggregate labels passed on the Bazel command
    /// line. Empty for the query-pattern-file candidate: Bazel reads the
    /// labels from [`RepositoryRootPlan::pattern_file`] instead.
    pub roots: Vec<String>,
    /// Query-produced label file for the pattern-file candidate, passed via
    /// [`PATTERN_FILE_FLAG`]. `None` for every other strategy.
    pub pattern_file: Option<PathBuf>,
}

impl RepositoryRootPlan {
    /// Plans the correctness baseline: aspects apply to `//...`.
    pub fn baseline() -> RepositoryRootPlan {
        RepositoryRootPlan {
            strategy: RootStrategy::RecursivePattern,
            roots: vec![REPOSITORY_PATTERN.to_owned()],
            pattern_file: None,
        }
    }

    /// Plans the query-pattern-file candidate: no command-line patterns,
    /// Bazel reads `path` (one label per line) through
    /// [`PATTERN_FILE_FLAG`].
    pub fn query_pattern_file(path: &Path) -> RepositoryRootPlan {
        RepositoryRootPlan {
            strategy: RootStrategy::QueryPatternFile,
            roots: Vec::new(),
            pattern_file: Some(path.to_owned()),
        }
    }

    /// Plans the monolithic-aggregate candidate: Bazel builds the one
    /// aggregate label, which must propagate aspect providers and expose a
    /// requested output group.
    pub fn monolithic_aggregate(label: &str) -> RepositoryRootPlan {
        RepositoryRootPlan {
            strategy: RootStrategy::MonolithicAggregate,
            roots: vec![label.to_owned()],
            pattern_file: None,
        }
    }

    /// Plans the package-shards candidate: Bazel builds one aggregate label
    /// per package.
    pub fn package_shards(labels: &[String]) -> RepositoryRootPlan {
        RepositoryRootPlan {
            strategy: RootStrategy::PackageShards,
            roots: labels.to_vec(),
            pattern_file: None,
        }
    }

    /// Renders the `--target_pattern_file` argument for the pattern-file
    /// candidate, or `None` when this plan carries no pattern file.
    pub fn pattern_file_arg(&self) -> Option<String> {
        self.pattern_file
            .as_ref()
            .map(|path| format!("{}={}", PATTERN_FILE_FLAG, path.display()))
    }
}

/// Plans the repository-wide roots behind a canonical selection
/// (`//dx:codegen`, `//dx:env`, or their union in `dx setup`): the frozen
/// WP4 strategy's plan. The freeze (see [`frozen_strategy`]) selects the
/// `//...` baseline, so this is the baseline plan; composing a future
/// non-baseline winner (pattern-file path, aggregate labels) needs its
/// invocation-time inputs and lands with that freeze change, not here.
/// Exact-target scopes bypass root selection entirely and never call this.
pub fn repository_plan() -> RepositoryRootPlan {
    debug_assert_eq!(frozen_strategy(), RootStrategy::baseline());
    RepositoryRootPlan::baseline()
}

/// Composes a root plan into the Bazel command-line patterns behind one
/// canonical repository-wide selection: the baseline plan keeps the
/// canonical selection identity (so `//dx:codegen` keeps resolving while
/// the benchmark runs); every other candidate passes its own roots
/// through. The query-pattern-file candidate carries no command-line
/// patterns: Bazel reads the labels from [`PATTERN_FILE_FLAG`] (see
/// [`RepositoryRootPlan::pattern_file_arg`]).
pub fn invocation_targets(plan: &RepositoryRootPlan, canonical: &str) -> Vec<String> {
    if plan.pattern_file.is_some() {
        return Vec::new();
    }
    if plan.strategy == RootStrategy::RecursivePattern
        && plan.roots == vec![REPOSITORY_PATTERN.to_owned()]
    {
        return vec![canonical.to_owned()];
    }
    plan.roots.clone()
}

/// Composes a root plan into a `bazel build` command line: `build` plus
/// [`invocation_targets`], one `--aspects=` flag per collecting aspect,
/// one `--output_groups=` flag per plan output group, and the
/// [`PATTERN_FILE_FLAG`] argument when the plan carries a pattern file.
pub fn build_argv(
    plan: &RepositoryRootPlan,
    canonical: &str,
    aspects: &[String],
    output_groups: &[String],
) -> Vec<String> {
    let mut argv = vec!["build".to_owned()];
    argv.extend(invocation_targets(plan, canonical));
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

/// Designated-root coverage report: whether a candidate's roots list every
/// semantic root the repository-wide selection must analyze. Comparison is
/// over sorted deduplicated label sets; Bazel deduplicates identical
/// configured targets downstream, so duplicate entries are inert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    /// Designated semantic roots absent from the candidate. Non-empty means
    /// the candidate reduces the selection (for example from an
    /// unconfigured query graph alone) and fails closed.
    pub missing: Vec<String>,
    /// Candidate roots outside the designated set. Tolerated: aspects stay
    /// provider-selective, so irrelevant targets contribute no plan records;
    /// reported so benchmarks can attribute discovery cost.
    pub extra: Vec<String>,
}

impl CoverageReport {
    /// True when the candidate covers every designated semantic root.
    pub fn is_covered(&self) -> bool {
        self.missing.is_empty()
    }
}

/// Designated-root coverage failure: the candidate drops required semantic
/// roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageError {
    /// Sorted deduplicated designated roots absent from the candidate.
    pub missing: Vec<String>,
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "root candidate drops {} designated root(s): {}",
            self.missing.len(),
            self.missing.join(", ")
        )
    }
}

impl std::error::Error for CoverageError {}

fn sorted_set(labels: &[String]) -> BTreeSet<String> {
    labels.iter().cloned().collect()
}

/// Checks that `candidate_roots` lists every one of `designated_roots`.
/// An empty candidate never covers a nonempty designated set: a
/// query-produced list must name every designated semantic root and let
/// Bazel deduplicate identical configured targets, never silently narrow
/// the selection. Extra candidate roots are tolerated and reported.
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

/// Benchmark dimensions every root candidate must report, per the codegen
/// performance model: cold and warm loading/analysis, source and BUILD
/// edits, target add/remove, generator actions, materialized bytes,
/// projection time, and retained memory.
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
    /// Every measured dimension, in stable report order.
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

    /// Stable machine-readable identity for benchmark rows.
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

/// One candidate's headline benchmark sample for the O34 decision. Full
/// per-dimension rows land with the measurement harness; the freeze rule
/// compares cold against warm only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkSample {
    /// Candidate strategy this sample measures.
    pub strategy: RootStrategy,
    /// Whether the candidate selected equivalent effective roots and
    /// produced equivalent outputs to the baseline run. Non-equivalent
    /// candidates never win, however fast.
    pub equivalent: bool,
    /// Cold Bazel time in milliseconds.
    pub cold_ms: u64,
    /// Warm Bazel time in milliseconds (persistent server, warm action
    /// cache), weighted above cold per O34.
    pub warm_ms: u64,
}

/// O34 decision score: `cold_ms + WARM_WEIGHT * warm_ms`, saturating. Warm
/// dominates, so a candidate trading slower cold for much faster warm can
/// win.
pub fn weighted_score(sample: &BenchmarkSample) -> u128 {
    u128::from(sample.cold_ms)
        .saturating_add(u128::from(sample.warm_ms).saturating_mul(u128::from(WARM_WEIGHT)))
}

/// Selects the repository-root strategy from headline samples: among
/// equivalent-semantics candidates the lowest weighted score wins, with
/// warm weighted above cold. Ties break toward the baseline-first
/// [`RootStrategy::ALL`] order, and an empty or fully non-equivalent sample
/// set keeps the `//...` baseline.
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
    fn benchmark_dimensions_are_named() {
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
