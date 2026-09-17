//! Single-version pin and rollback for `dx version` (issue #236).
//!
//! Split from `super` (`lib.rs`): owns `DX_VERSION`, `MODULE_VERSION`,
//! `PREVIOUS_VERSION`, `version_pin_matches_module`,
//! `rollback_re_pins_previous`, `read_version_pin`, and
//! `write_version_pin`. Re-exported through `super` so the public path
//! stays `dx_adopt::{DX_VERSION, MODULE_VERSION, PREVIOUS_VERSION,
//! version_pin_matches_module, rollback_re_pins_previous,
//! read_version_pin, write_version_pin}`.

use std::path::Path;

use super::AdoptError;

/// Delivered `dx` / `rules_dx` single version (O51 freeze).
pub const DX_VERSION: &str = "0.0.0";
/// Pinned `rules_dx` module version; `dx version` must equal this.
pub const MODULE_VERSION: &str = "0.0.0";
/// Previous release for rollback demonstration.
pub const PREVIOUS_VERSION: &str = "0.0.0";

/// Whether the single-version pin holds.
///
/// Per O51 direction the `dx` version equals the pinned `rules_dx` module
/// version: both must parse as Cargo-flavor semver (via the `semver`
/// crate, issue #224) and compare exactly equal. Self-update bumps
/// that pin from verified release artifacts; anything else is rejected here.
/// Empty strings and non-semver text never match, even when equal.
pub fn version_pin_matches_module(dx_version: &str, module_version: &str) -> bool {
    if dx_version.is_empty() || module_version.is_empty() {
        return false;
    }
    let dx = match semver::Version::parse(dx_version) {
        Ok(dx) => dx,
        Err(_) => return false,
    };
    let module = match semver::Version::parse(module_version) {
        Ok(module) => module,
        Err(_) => return false,
    };
    dx == module
}

/// Whether a rollback target is admissible.
///
/// Rollback is re-pinning the previous release: the target must equal the
/// known previous version and differ from the current pin. Rolling to the
/// current pin or to an unknown version is rejected.
pub fn rollback_re_pins_previous(current: &str, target: &str, known_previous: &str) -> bool {
    !known_previous.is_empty() && target == known_previous && target != current
}

/// Read the `.dx/version` pin under `root`.
pub fn read_version_pin(root: &Path) -> Result<String, AdoptError> {
    let raw = std::fs::read_to_string(root.join(".dx/version")).map_err(|e| {
        AdoptError::ReadVersionPin {
            detail: e.to_string(),
        }
    })?;
    Ok(raw.trim().to_owned())
}

/// Write the `.dx/version` pin (verified-release versions only).
pub fn write_version_pin(root: &Path, version: &str) -> Result<(), AdoptError> {
    if version.is_empty() {
        return Err(AdoptError::EmptyVersion);
    }
    let dir = root.join(".dx");
    std::fs::create_dir_all(&dir).map_err(|e| AdoptError::CreateDxDir {
        detail: e.to_string(),
    })?;
    dx_atomic_fs::write_atomic(&dir.join("version"), format!("{version}\n").as_bytes()).map_err(
        |e| AdoptError::WriteVersionPin {
            detail: e.to_string(),
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_holds_only_on_equal_nonempty_versions() {
        assert!(version_pin_matches_module("1.2.3", "1.2.3"));
        assert!(!version_pin_matches_module("1.2.3", "1.2.4"));
        assert!(!version_pin_matches_module("", ""));
        assert!(!version_pin_matches_module("1.2.3", ""));
        // Issue #224 (semver pilot): pins must be valid semver; equal
        // non-semver text never matches, even when verbatim equal.
        assert!(!version_pin_matches_module("abc", "abc"));
        assert!(!version_pin_matches_module("v1.2.3", "v1.2.3"));
        assert!(!version_pin_matches_module("1.2", "1.2"));
        // Pre-release and build metadata compare exactly.
        assert!(version_pin_matches_module("1.2.3-alpha.1", "1.2.3-alpha.1"));
        assert!(!version_pin_matches_module(
            "1.2.3-alpha.1",
            "1.2.3-alpha.2"
        ));
        assert!(!version_pin_matches_module("1.2.3", "1.2.3-alpha.1"));
    }

    #[test]
    fn rollback_re_pins_only_the_known_previous() {
        assert!(rollback_re_pins_previous("1.2.4", "1.2.3", "1.2.3"));
        assert!(!rollback_re_pins_previous("1.2.3", "1.2.3", "1.2.3"));
        assert!(!rollback_re_pins_previous("1.2.4", "1.2.2", "1.2.3"));
        assert!(!rollback_re_pins_previous("1.2.4", "", ""));
    }

    #[test]
    fn delivered_version_matches_module() {
        assert!(version_pin_matches_module(DX_VERSION, MODULE_VERSION));
        // Single-version state (no releases cut): the rollback target
        // equals the delivered version, so rollback correctly refuses.
        assert!(!rollback_re_pins_previous(
            DX_VERSION,
            PREVIOUS_VERSION,
            PREVIOUS_VERSION
        ));
    }
}
