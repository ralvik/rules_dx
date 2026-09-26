use std::collections::BTreeMap;

use super::LcovError;

pub const ELIGIBLE: &str = "eligible";
pub const SUPPORT: &str = "support";

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
