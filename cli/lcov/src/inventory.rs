//! Repo inventory for the coverage gate.
//!
//! Split from `super` (`lib.rs`): owns the inventory dispositions
//! ([`ELIGIBLE`], [`SUPPORT`]) and [`parse_inventory`] (the
//! `<disposition> <path>` file parser). Re-exported through `super` so
//! the public paths stay `dx_lcov::{ELIGIBLE, SUPPORT, parse_inventory}`.
//! Distinct from the `parse` module (combined-LCOV parsing), the
//! `ignores` module (source-level exclusion markers), the `verdict`
//! module (gate evaluation), and the `run` module (gate CLI).

use std::collections::BTreeMap;

use super::LcovError;

/// Inventory disposition for authored first-party implementation.
pub const ELIGIBLE: &str = "eligible";
/// Inventory disposition for classified non-implementation (build
/// declarations, schemas, fixtures, tooling inputs). Never in the denominator.
pub const SUPPORT: &str = "support";

/// Parse the inventory file: `<disposition> <repo-relative path>` per line;
/// blank lines and `#` comments are skipped.
pub fn parse_inventory(text: &str) -> Result<BTreeMap<String, String>, LcovError> {
    let mut inventory = BTreeMap::new();
    for (index, raw) in text.lines().enumerate() {
        let lineno = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let disposition = parts.next().unwrap_or_default();
        let path = parts.next().unwrap_or_default();
        if disposition.is_empty() || path.is_empty() || parts.next().is_some() {
            return Err(LcovError::MalformedInventory {
                lineno,
                raw: raw.to_string(),
            });
        }
        inventory.insert(path.to_string(), disposition.to_string());
    }
    Ok(inventory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inventory_with_comments_and_blanks() {
        let inventory = parse_inventory("# comment\n\neligible a.rs\nsupport b.rs  \n").unwrap();
        assert_eq!(inventory["a.rs"], ELIGIBLE);
        assert_eq!(inventory["b.rs"], SUPPORT);
    }

    #[test]
    fn rejects_malformed_inventory_lines() {
        assert!(parse_inventory("eligible\n").is_err());
        assert!(parse_inventory("eligible a.rs extra\n").is_err());
        assert!(parse_inventory("   \n lone\n").is_err());
    }
}
