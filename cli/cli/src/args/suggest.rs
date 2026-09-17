//! Typo-suggestion helpers for the `dx` CLI (issue #236).
//!
//! Split from `super` (`args.rs`): owns [`best_match`],
//! [`suggest_command`], [`suggest_option`], [`clap_suggestion`], and
//! [`clap_command_suggestion`]. Internal only (`pub(crate)`); the public
//! `crate::args` surface is unchanged.

use super::command::Command;
use super::parser::Cli;

/// Best candidate above clap's confidence bar. Mirrors
/// `clap_builder::parser::features::suggestions::did_you_mean` (same
/// upstream `strsim::jaro` + `0.7` threshold): that helper is
/// crate-private, so the typo path re-applies its rule over our own
/// candidate list instead of hand-rolling edit distance.
pub(crate) fn best_match(
    input: &str,
    candidates: impl Iterator<Item = impl AsRef<str>>,
) -> Option<String> {
    let mut best: Option<(f64, String)> = None;
    for candidate in candidates {
        let confidence = strsim::jaro(input, candidate.as_ref());
        if confidence > 0.7 && best.as_ref().is_none_or(|(score, _)| confidence > *score) {
            best = Some((confidence, candidate.as_ref().to_owned()));
        }
    }
    best.map(|(_, name)| name)
}

/// Suggests the closest command word from the [`Command`] derive (the
/// single grammar source), so the hint can never drift from the
/// accepted spellings.
pub(crate) fn suggest_command(input: &str) -> Option<String> {
    use clap::ValueEnum;
    best_match(
        input,
        Command::value_variants().iter().copied().map(Command::name),
    )
}

/// Suggests the closest `--flag` for an offending token. Candidates
/// come from the clap grammar (`Cli::command()` longs), so the hint
/// tracks `Cli` renames without a second list. Exact matches yield no
/// hint (the flag is right; the `=value` is wrong), and non-flag
/// tokens yield none.
pub(crate) fn suggest_option(token: &str) -> Option<String> {
    use clap::CommandFactory;
    let name = token.split(['=', ' ', '\t']).next().unwrap_or(token);
    let bare = name.strip_prefix("--").or_else(|| name.strip_prefix('-'))?;
    if bare.is_empty() {
        return None;
    }
    // Single-character `-q`-style tokens never suggest: jaro("q","quiet")
    // clears 0.7 but a one-letter flag is a missing-short attempt, not a
    // `--long` typo. Longer typos (`--ouptut`) still flow to best_match.
    if bare.len() < 2 {
        return None;
    }
    let cmd = Cli::command();
    let longs: Vec<&str> = cmd
        .get_arguments()
        .filter_map(|arg| arg.get_long())
        .collect();
    if longs.contains(&bare) {
        return None;
    }
    best_match(bare, longs.into_iter()).map(|hit| format!("--{hit}"))
}

/// Reads clap's own suggestion off a parse failure (`--flag` render),
/// when the unknown flag is close enough for clap to name one.
pub(crate) fn clap_suggestion(error: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};
    for kind in [ContextKind::SuggestedArg, ContextKind::Suggested] {
        match error.get(kind) {
            Some(ContextValue::String(hit)) => return Some(hit.clone()),
            Some(ContextValue::Strings(hits)) => {
                if let Some(hit) = hits.first() {
                    return Some(hit.clone());
                }
            }
            _ => {}
        }
    }
    None
}

/// Reads clap's own command suggestion off a `ValueEnum` positional
/// rejection (same `jaro` rule as [`best_match`], computed by clap over
/// the [`Command`] variants).
pub(crate) fn clap_command_suggestion(error: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};
    match error.get(ContextKind::SuggestedValue) {
        Some(ContextValue::String(hit)) => Some(hit.clone()),
        Some(ContextValue::Strings(hits)) => hits.first().cloned(),
        _ => None,
    }
}
