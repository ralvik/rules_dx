//! One-shot `dx upgrade`: pin plus migrate plus setup composition.
//!
//! Split from `super` (`lib.rs`): owns `UpgradePlan`, `plan_upgrade`,
//! plus the manual recovery helpers (`upgrade_retry_command`,
//! `upgrade_restore_command`, `upgrade_recovery_message`). Re-exported
//! through `super` so the public path stays
//! `dx_adopt::{plan_upgrade, UpgradePlan, ...}`.
//!
//! Composition only, no new version semantics: the pin step reuses the
//! single-version pin (`dx version`), the migrate step reuses
//! [`super::plan_migrate`] (upgrade-only gate plus mechanical manifest
//! selection), and the setup step is the atomic `dx setup` selection.
//! Live execution fails closed like `dx migrate` until the first manifest
//! lands (module at `0.0.0`, no releases cut); recovery is manual plus
//! idempotent retry, mirroring `dx update` per-set recovery.
//! See: `docs/cli/commands/new-upgrade.md`.

use super::AdoptError;

/// One planned upgrade: the validated version pair plus the migrate
/// manifest plus the pin target plus manual recovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradePlan {
    /// Source version (inclusive, already installed).
    pub from: String,
    /// Target version (also the `.dx/version` pin target).
    pub to: String,
    /// Selected migrate manifest (see [`super::migrate_manifest_name`]).
    pub manifest: String,
    /// Idempotent retry invocation.
    pub retry_command: String,
    /// Manual restore invocation for the pin when present.
    pub restore_command: Option<String>,
    /// Human-readable one-line recovery hint shared by text and JSON.
    pub message: String,
}

/// Idempotent retry invocation for an upgrade pair.
pub fn upgrade_retry_command(from: &str, to: &str) -> String {
    format!("dx upgrade --from {from} --to {to}")
}

/// Manual restore invocation for the version pin.
///
/// The command never runs inside `dx` (Git inspection stays rejected);
/// it is printed for the operator to run when the tree is
/// version-controlled. `Some` always: the pin is the one mutable step
/// that may have landed before a later failure.
pub fn upgrade_restore_command() -> Option<String> {
    Some("git checkout -- .dx/version".to_owned())
}

/// Human-readable one-line recovery hint shared by text and JSON.
pub fn upgrade_recovery_message(plan: &UpgradePlan) -> String {
    match &plan.restore_command {
        Some(restore) => format!(
            "recovery: rerun `{}` (idempotent); to discard a landed pin run `{restore}` then rerun setup with `dx setup`",
            plan.retry_command,
        ),
        None => format!(
            "recovery: rerun `{}` (idempotent)",
            plan.retry_command,
        ),
    }
}

/// Plan one `dx upgrade` invocation.
///
/// Syntax: `dx upgrade --from <version> --to <version>`. Both versions
/// are Cargo-flavor semver; the pair must be an upgrade (see
/// [`super::migrate_is_upgrade`]). Manifest selection matches
/// [`super::plan_migrate`] verbatim (one per major hop, one per full
/// version pair for minor/patch). With no manifests published yet,
/// planning succeeds but execution fails closed (`upgrade_failed`)
/// until the first manifest lands.
pub fn plan_upgrade(from: &str, to: &str) -> Result<UpgradePlan, AdoptError> {
    let migrate = super::plan_migrate(from, to)?;
    let retry_command = upgrade_retry_command(from, to);
    let restore_command = upgrade_restore_command();
    let mut plan = UpgradePlan {
        from: from.to_owned(),
        to: to.to_owned(),
        manifest: migrate.manifest,
        retry_command,
        restore_command,
        message: String::new(),
    };
    plan.message = upgrade_recovery_message(&plan);
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_reuses_migrate_gate_and_manifest() {
        // See: `docs/cli/commands/new-upgrade.md`.
        let plan = plan_upgrade("1.2.3", "2.0.0").expect("major plans");
        assert_eq!(plan.from, "1.2.3");
        assert_eq!(plan.to, "2.0.0");
        assert_eq!(plan.manifest, "migrate-v1-to-v2.json");
        assert_eq!(plan.retry_command, "dx upgrade --from 1.2.3 --to 2.0.0");
        assert_eq!(
            plan.restore_command,
            Some("git checkout -- .dx/version".to_owned())
        );
        assert!(plan.message.contains("dx upgrade --from 1.2.3 --to 2.0.0"));
        assert!(plan.message.contains("idempotent"));
        assert!(plan.message.contains("dx setup"));
        let minor = plan_upgrade("1.2.3", "1.3.0").expect("minor plans");
        assert_eq!(minor.manifest, "migrate-v1.2.3-to-v1.3.0.json");
        assert!(plan_upgrade("2.0.0", "1.0.0").is_err());
        assert_eq!(
            plan_upgrade("2.0.0", "1.0.0").unwrap_err().to_string(),
            "migrate is upgrade-only: 2.0.0 -> 1.0.0"
        );
        assert_eq!(
            plan_upgrade("", "2.0.0").unwrap_err().to_string(),
            "migrate needs distinct versions: from and to must both be set"
        );
    }

    #[test]
    fn upgrade_recovery_points_at_retry_plus_pin_restore() {
        let plan = plan_upgrade("1.2.3", "2.0.0").expect("plans");
        assert_eq!(
            upgrade_retry_command("1.2.3", "2.0.0"),
            "dx upgrade --from 1.2.3 --to 2.0.0"
        );
        assert_eq!(
            upgrade_restore_command(),
            Some("git checkout -- .dx/version".to_owned())
        );
        let message = upgrade_recovery_message(&plan);
        assert!(message.contains("rerun"));
        assert!(message.contains("git checkout -- .dx/version"));
    }
}
