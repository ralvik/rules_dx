//! Flag shapes for `dx clean`.
//!
//! Split from `super` (`lib.rs`): owns [`DRY_RUN_FLAG`], [`BAZEL_FLAG`],
//! [`RECOVERY_GUIDANCE`], and [`bazel_forward_argv`] (the frozen
//! `dx clean [--dry-run] [--bazel]` shapes plus the `bazel clean`
//! forwarding shape). Re-exported through `super` so the public paths
//! stay `dx_clean::{DRY_RUN_FLAG, BAZEL_FLAG, RECOVERY_GUIDANCE,
//! bazel_forward_argv}`. Distinct from the record/planning, inventory,
//! apply, and bytes modules.

/// `--dry-run` flag: list reclaimable generations and links without
/// deleting. Matches the `dx clean` contract; frozen here so CLI
/// parsing and help text cannot drift from the qualified shape.
pub const DRY_RUN_FLAG: &str = "--dry-run";

/// `--bazel` flag: additionally forward `bazel clean` and print
/// [`RECOVERY_GUIDANCE`]. Explicit opt-in only; default `dx clean`
/// never touches Bazel outputs.
pub const BAZEL_FLAG: &str = "--bazel";

/// Recovery guidance printed after an explicit `dx clean --bazel`
/// forward, per the clean contract: `bazel clean` leaves managed links
/// dangling, and only explicit `dx env` / `dx codegen` / `dx setup`
/// repairs the projection. Links never self-repair.
pub const RECOVERY_GUIDANCE: &str = "bazel clean forwarded; managed links may now dangle: \
    re-run `dx setup` (or `dx env` / `dx codegen`) to repair the selection";

/// `bazel` subcommand forwarded by `dx clean --bazel`: exactly
/// `bazel clean`, never any other Bazel verb.
pub fn bazel_forward_argv() -> Vec<String> {
    vec!["clean".to_owned()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_shape_is_frozen() {
        assert_eq!(DRY_RUN_FLAG, "--dry-run");
        assert_eq!(BAZEL_FLAG, "--bazel");
        assert_eq!(bazel_forward_argv(), vec!["clean".to_owned()]);
        assert!(RECOVERY_GUIDANCE.contains("dx setup"));
    }
}
