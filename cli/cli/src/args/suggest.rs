use super::command::Command;
use super::grammar::Cli;

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

pub(crate) fn suggest_command(input: &str) -> Option<String> {
    if input.eq_ignore_ascii_case("doctor") || input.eq_ignore_ascii_case("configure") {
        return Some("status".to_owned());
    }
    use clap::ValueEnum;
    best_match(
        input,
        Command::value_variants().iter().copied().map(Command::name),
    )
}

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

pub(crate) fn clap_command_suggestion(error: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};
    match error.get(ContextKind::SuggestedValue) {
        Some(ContextValue::String(hit)) => Some(hit.clone()),
        Some(ContextValue::Strings(hits)) => hits.first().cloned(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{parse, ArgsError};

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn typo_recovery_suggests_commands_and_options() {
        assert_eq!(
            parse(&args(&["lintt"])),
            Err(ArgsError::UnknownCommand {
                command: "lintt".to_owned(),
                suggestion: Some("lint".to_owned()),
            })
        );
        assert_eq!(
            parse(&args(&["--ouptut=json"])),
            Err(ArgsError::UnknownOption {
                option: "--ouptut=json".to_owned(),
                suggestion: Some("--output".to_owned()),
            })
        );
        let command = parse(&args(&["lintt"])).unwrap_err().to_string();
        assert!(command.contains("unknown command \"lintt\""));
        assert!(command.contains("did you mean \"lint\"?"));
        let option = parse(&args(&["--ouptut=json"])).unwrap_err().to_string();
        assert!(option.contains("unknown option \"--ouptut=json\""));
        assert!(option.contains("did you mean \"--output\"?"));
    }

    #[test]
    fn excluded_doctor_and_configure_redirect_to_status() {
        for word in ["doctor", "configure"] {
            assert_eq!(
                parse(&args(&[word])),
                Err(ArgsError::UnknownCommand {
                    command: word.to_owned(),
                    suggestion: Some("status".to_owned()),
                }),
                "word: {word}"
            );
            let text = parse(&args(&[word])).unwrap_err().to_string();
            assert!(
                text.contains("did you mean \"status\"?"),
                "word: {word}: {text}"
            );
        }
        for word in ["doctor", "configure"] {
            assert_eq!(
                parse(&args(&["help", word])),
                Err(ArgsError::UnknownCommand {
                    command: word.to_owned(),
                    suggestion: Some("status".to_owned()),
                }),
                "help {word}"
            );
        }
    }
}
