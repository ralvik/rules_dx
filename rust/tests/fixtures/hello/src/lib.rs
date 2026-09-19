//! M00 seed library: minimal greeting logic plus an anyhow error path
//! exercising the external-dependency fixture.

use anyhow::{Context, Result};

/// Returns a greeting for `name`. Empty names are rejected so the error
/// path stays covered by tests.
pub fn greet(name: &str) -> Result<String> {
    if name.is_empty() {
        anyhow::bail!("name must not be empty");
    }
    Ok(format!("Hello, {name}!"))
}

/// Adds two integers with documented wrapping semantics.
pub fn add(left: u64, right: u64) -> u64 {
    left.wrapping_add(right)
}

/// Parses `s` as a u64, attaching context so the anyhow dependency is
/// exercised beyond attribute macros.
pub fn parse_count(s: &str) -> Result<u64> {
    s.parse::<u64>()
        .with_context(|| format!("failed to parse count from {s:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_by_name() {
        assert_eq!(greet("world").unwrap(), "Hello, world!");
    }

    #[test]
    fn rejects_empty_name() {
        assert!(greet("").is_err());
    }

    #[test]
    fn adds_numbers() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(u64::MAX, 1), 0);
    }

    #[test]
    fn parses_counts() {
        assert_eq!(parse_count("42").unwrap(), 42);
        assert!(parse_count("nope").is_err());
    }
}
