use std::path::Path;

use super::AdoptError;

pub const DX_VERSION: &str = "0.0.0";
pub const MODULE_VERSION: &str = "0.0.0";
pub const PREVIOUS_VERSION: &str = "0.0.0";

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

pub fn rollback_re_pins_previous(current: &str, target: &str, known_previous: &str) -> bool {
    !known_previous.is_empty() && target == known_previous && target != current
}

pub fn read_version_pin(root: &Path) -> Result<String, AdoptError> {
    let raw = std::fs::read_to_string(root.join(".dx/version")).map_err(|e| {
        AdoptError::ReadVersionPin {
            detail: e.to_string(),
        }
    })?;
    Ok(raw.trim().to_owned())
}

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
        assert!(!version_pin_matches_module("abc", "abc"));
        assert!(!version_pin_matches_module("v1.2.3", "v1.2.3"));
        assert!(!version_pin_matches_module("1.2", "1.2"));
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
        assert!(!rollback_re_pins_previous(
            DX_VERSION,
            PREVIOUS_VERSION,
            PREVIOUS_VERSION
        ));
    }
}
