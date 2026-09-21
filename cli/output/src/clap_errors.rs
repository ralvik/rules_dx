//! Shared `clap` tokenizing-failure primitives (frozen legacy contract).
//!
//! See: `docs/cli/cli-contract.md` for the unknown-flag/missing-value
//! message shapes. The thin shims (`generation/codegen_shard`,
//! `env/env_shard`, `cli/env`, `cli/lcov`, `quality/evaluator`,
//! `quality/runner`, `quality/markdown`, `cli/cli`) keep their own
//! `parse_error` message formats (usage routing, `unknown flag` vs
//! `unknown argument`, per-flag value parsers); only the context reads
//! below are shared so the `InvalidArg`/`InvalidValue` plumbing has one
//! owner.

// Infallible paths must not `expect`/`unwrap` outside tests
// (`cfg_attr(not(test))` keeps `rust_test` bodies ergonomic).
#![cfg_attr(not(test), deny(clippy::expect_used, clippy::unwrap_used))]

/// Raw `argv` token behind a [`clap::Error`], e.g. `--bogus` or `oops`.
/// Empty when the error carries no invalid-argument context.
pub fn invalid_token(error: &clap::Error) -> String {
    match error.get(clap::error::ContextKind::InvalidArg) {
        Some(clap::error::ContextValue::String(token)) => token.clone(),
        Some(clap::error::ContextValue::Strings(tokens)) => {
            tokens.first().cloned().unwrap_or_default()
        }
        _ => String::new(),
    }
}

/// Rejected option value behind a [`clap::Error`], if the error carries a
/// non-empty one. Missing values carry none (or an empty one), which the
/// caller treats as missing rather than rejected.
pub fn rejected_value(error: &clap::Error) -> Option<String> {
    let invalid = error.get(clap::error::ContextKind::InvalidValue)?;
    let raw = match invalid {
        clap::error::ContextValue::String(value) => value.clone(),
        clap::error::ContextValue::Strings(values) => values.first().cloned().unwrap_or_default(),
        _ => String::new(),
    };
    if raw.is_empty() {
        None
    } else {
        Some(raw)
    }
}

/// Recovers the exact offending `argv` element for an unknown option:
/// `clap` strips an attached `=value` from the reported token while the
/// legacy loop echoed the whole `argv` element.
pub fn recover_unknown_token(args: &[String], token: &str) -> String {
    args.iter()
        .find(|arg| *arg == token)
        .or_else(|| {
            args.iter()
                .find(|arg| arg.starts_with(&format!("{token}=")))
        })
        .map_or(token.to_owned(), Clone::clone)
}

/// Extracts the leading `--flag` from a `clap` missing-value render such
/// as `--output <OUTPUT>`; the legacy message names the bare `--flag`.
pub fn leading_flag(token: &str) -> &str {
    token.split_whitespace().next().unwrap_or(token)
}

/// First line of a `clap` error render, for the unreachable fallback arm.
pub fn first_line(error: &clap::Error) -> String {
    error
        .to_string()
        .lines()
        .next()
        .unwrap_or("invalid arguments")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_flag_names_the_bare_flag() {
        assert_eq!(leading_flag("--output <OUTPUT>"), "--output");
        assert_eq!(leading_flag("--report"), "--report");
        assert_eq!(leading_flag(""), "");
    }

    #[test]
    fn recover_unknown_token_echoes_attached_values() {
        let args = vec!["--output=x".to_owned(), "--bogus".to_owned()];
        assert_eq!(recover_unknown_token(&args, "--output"), "--output=x");
        assert_eq!(recover_unknown_token(&args, "--bogus"), "--bogus");
        assert_eq!(recover_unknown_token(&args, "--missing"), "--missing");
    }
}
