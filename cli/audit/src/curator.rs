//! Audit curator data as declared Bazel inputs.
//!
//! Pure planning for the declared-inputs delivery of the audit contract
//! (`docs/cli/commands/audit-update-bazel.md#dx-audit`): the committed
//! `licenses.toml` policy plus per-package `[[inventory]]` shape stays
//! identical, while Bazel-owned analysis declares the same bytes as
//! inputs so results and cache identity reflect the data actually
//! analyzed rather than an untracked workspace read. Advisory snapshots
//! ride the same contract via [`crate::advisory::snapshot_rel`] plus
//! [`crate::advisory::identity_rel`]; they are acquired, not committed,
//! so no filegroup lists them here.
//!
//! This module pins the label plus relative-path mapping only, so the
//! CLI's workspace reads and Bazel-owned analysis agree on one identity
//! without a second mechanism. Per-package inventory shape
//! (`package`, `set`, `license`, `versions`, `text_present`) is owned by
//! [`crate::license_policy::LicenseInventory`]; NOTICE aggregation over
//! these same inputs stays hermetic and cached in
//! `deploy/release/notice.bzl`.
//!
//! See: `docs/cli/commands/audit-update-bazel.md#dx-audit`.

/// Bazel label exposing the committed curator file as a declared input.
/// The target lives in the root package (`//:audit_curator` over
/// `licenses.toml`); the CLI keeps reading the same workspace-relative
/// bytes via [`LICENSES_TOML_REL`], so both paths analyze identical data.
pub const LICENSES_TOML_LABEL: &str = "//:audit_curator";

/// Workspace-relative path of the committed curator file. The CLI reads
/// this path; Bazel-owned analysis declares [`LICENSES_TOML_LABEL`] as
/// an input carrying the same bytes.
pub const LICENSES_TOML_REL: &str = "licenses.toml";

/// Dependency sets owning identified advisory snapshots as declared
/// analysis inputs. Spelling matches [`crate::advisory::advisory_source`]
/// plus [`crate::backend::vuln_locks`], so audit and update agree on
/// owning sets without a second registry.
pub const CURATOR_ADVISORY_SETS: &[&str] = &["cargo", "npm", "maven", "nuget", "go"];

/// Declared-input relative paths for one advisory set: the snapshot bytes
/// plus the identity document (`url`, `sha256`, `retrieved_at`). Both
/// ride Bazel-owned analysis as inputs; a missing, invalid, or stale
/// snapshot fails with [`crate::advisory::CODE_ADVISORY_REFRESH_FAILED`],
/// never clean and never a stale fallback.
pub fn advisory_inputs(set: &str) -> [String; 2] {
    [
        crate::advisory::snapshot_rel(set),
        crate::advisory::identity_rel(set),
    ]
}

/// All declared curator input rels: the committed policy file plus every
/// set's snapshot bytes plus identity. Order is deterministic
/// (`licenses.toml` first, then per-set snapshot plus identity in
/// [`CURATOR_ADVISORY_SETS`] order) so action keys stay stable.
pub fn curator_input_rels() -> Vec<String> {
    let mut out = vec![LICENSES_TOML_REL.to_owned()];
    for set in CURATOR_ADVISORY_SETS {
        out.extend(advisory_inputs(set));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curator_label_and_rel_are_pinned() {
        assert_eq!(LICENSES_TOML_LABEL, "//:audit_curator");
        assert_eq!(LICENSES_TOML_REL, "licenses.toml");
    }

    #[test]
    fn advisory_sets_cover_all_registry_sets() {
        assert_eq!(
            CURATOR_ADVISORY_SETS,
            &["cargo", "npm", "maven", "nuget", "go"]
        );
        for set in CURATOR_ADVISORY_SETS {
            assert!(
                crate::advisory::advisory_source(set).is_some(),
                "{set} needs an advisory source"
            );
            assert!(
                !crate::backend::vuln_locks(set).is_empty(),
                "{set} needs lock coverage"
            );
        }
    }

    #[test]
    fn advisory_inputs_name_snapshot_plus_identity() {
        assert_eq!(
            advisory_inputs("cargo"),
            [
                ".dx/advisory/cargo.json".to_owned(),
                ".dx/advisory/cargo.meta.json".to_owned(),
            ]
        );
        assert_eq!(
            advisory_inputs("go"),
            [
                ".dx/advisory/go.json".to_owned(),
                ".dx/advisory/go.meta.json".to_owned(),
            ]
        );
    }

    #[test]
    fn curator_rels_are_deterministic_and_complete() {
        let rels = curator_input_rels();
        assert_eq!(rels.first().map(String::as_str), Some("licenses.toml"));
        assert_eq!(rels.len(), 1 + CURATOR_ADVISORY_SETS.len() * 2);
        // Deterministic order: snapshot plus identity per set in set order.
        let mut want = vec!["licenses.toml".to_owned()];
        for set in CURATOR_ADVISORY_SETS {
            want.push(format!(".dx/advisory/{set}.json"));
            want.push(format!(".dx/advisory/{set}.meta.json"));
        }
        assert_eq!(rels, want);
    }

    #[test]
    fn inventory_shape_stays_per_package_for_notice_aggregation() {
        // The future declared-inputs delivery keeps the same per-package
        // shape so NOTICE aggregation stays hermetic and cached: each
        // entry names its owning set, package, SPDX license, upstream
        // version scope, and whether the archive delivered words.
        let entry = crate::license_policy::LicenseInventory {
            package: "react".to_owned(),
            set: "npm".to_owned(),
            license: "MIT".to_owned(),
            versions: "18.2.0".to_owned(),
            text_present: true,
        };
        crate::license_policy::validate_license_inventory(&entry).expect("valid");
        assert!(entry.text_present);
    }
}
