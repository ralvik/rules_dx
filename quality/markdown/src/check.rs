use std::collections::{BTreeMap, BTreeSet};

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::Parser as ClapParser;
use pulldown_cmark::{BrokenLink, Event, LinkType, Options, Parser, Tag, TagEnd};
use serde::Serialize;

use super::frontmatter::{
    check_headings, code_regions, heading, is_closed_fence, line_of, line_starts, slug, Heading,
};
use super::links::{broken_label, check_target, scan_bare_autolinks, BrokenRef, DocLink, Span};

/// Structural finding kinds. Every variant is a finding; skipped remote
/// targets are reported separately in [`CheckOutcome::skipped_remotes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingKind {
    MissingFileTarget,
    MissingAnchor,
    HeadingHierarchy,
    MissingCodeFenceLanguage,
    UnclosedCodeFence,
}

/// One structural finding at a 1-based source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub line: u32,
    pub kind: FindingKind,
    pub message: String,
}

/// Outcome of [`check_markdown`]: findings ordered by line plus deduplicated
/// remote targets that were recorded but never fetched.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckOutcome {
    pub findings: Vec<Finding>,
    pub skipped_remotes: Vec<String>,
}

/// Markdown checker failure.
///
/// The injected file reader surfaces I/O failures verbatim so CLI
/// diagnostics stay byte-identical while callers gain a matchable type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MarkdownError {
    /// Injected file read failed; carries the reader's message verbatim.
    #[error("{message}")]
    Io { message: String },
}

impl From<String> for MarkdownError {
    fn from(message: String) -> Self {
        Self::Io { message }
    }
}

/// Check one Markdown source.
///
/// `source_path` is the source's repo-relative path; relative link targets
/// resolve against its parent directory. `siblings` maps repo-relative paths
/// to file contents for files a relative target may resolve to (the source
/// itself is never read from `siblings`).
pub fn check_markdown(
    source_path: &str,
    text: &str,
    siblings: &BTreeMap<String, String>,
) -> CheckOutcome {
    let mut outcome = CheckOutcome::default();
    let mut headings: Vec<Heading> = Vec::new();
    let mut doc_links: Vec<DocLink> = Vec::new();
    let mut slug_counts: BTreeMap<String, usize> = BTreeMap::new();

    // Block structure comes from pulldown-cmark: fence findings, suppressed
    // lines (code regions plus HTML blocks), headings, links, and code-span
    // ranges. Reference definitions resolve inside the parser; only
    // explicitly broken references surface via the callback.
    let starts = line_starts(text);
    let regions = code_regions(text, &starts);
    let source_lines: Vec<&str> = text.lines().collect();
    let mut suppressed: BTreeSet<u32> = BTreeSet::new();
    for region in &regions {
        for line in region.start_line..=region.end_line {
            suppressed.insert(line);
        }
        let Some(info) = region.fence_info.as_deref() else {
            continue;
        };
        if info.trim().is_empty() {
            push_finding(
                &mut outcome,
                region.start_line,
                FindingKind::MissingCodeFenceLanguage,
                "fenced code block without a language tag",
            );
        }
        if !is_closed_fence(&source_lines, region) {
            push_finding(
                &mut outcome,
                region.start_line,
                FindingKind::UnclosedCodeFence,
                "fenced code block opened here is never closed",
            );
        }
    }

    let mut heading_lines: Vec<u32> = Vec::new();
    let mut code_spans: Vec<Span> = Vec::new();
    let mut link_spans: Vec<Span> = Vec::new();
    let mut html_spans: Vec<Span> = Vec::new();
    let mut html_start: Option<usize> = None;
    let mut broken: Vec<BrokenRef> = Vec::new();
    {
        let callbacks = &mut |link: BrokenLink| {
            broken.push(BrokenRef {
                span: link.span.clone(),
                link_type: link.link_type,
                reference: link.reference.to_string(),
            });
            None
        };
        let parser = Parser::new_with_broken_link_callback(text, Options::empty(), Some(callbacks));
        for (event, range) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::Heading { .. }) => {
                    heading_lines.push(line_of(&starts, range.start));
                }
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    ..
                })
                | Event::Start(Tag::Image {
                    link_type,
                    dest_url,
                    ..
                }) => {
                    // Email autolinks are wont-fix out of scope:
                    // structure only, so ignored, never findings or remotes.
                    if link_type == LinkType::Email {
                        continue;
                    }
                    let at = line_of(&starts, range.start);
                    link_spans.push(range.clone());
                    doc_links.push(DocLink::Dest {
                        line: at,
                        col: range.start,
                        target: dest_url.to_string(),
                    });
                }
                Event::Code(_) => {
                    code_spans.push(range);
                }
                // Only real HTML blocks suppress lines: inline HTML leaves
                // the line's links (and relative autolinks) visible, exactly
                // as the retired line scanner saw them.
                Event::Start(Tag::HtmlBlock) => {
                    html_start = Some(range.start);
                }
                Event::End(TagEnd::HtmlBlock) => {
                    if let Some(start) = html_start.take() {
                        for line in line_of(&starts, start)..=line_of(&starts, range.end.max(1) - 1)
                        {
                            suppressed.insert(line);
                        }
                    }
                }
                // Inline HTML is markup, never an autolink target: record
                // its span so the relative-autolink fallback skips it.
                Event::Html(_) | Event::InlineHtml(_) => {
                    html_spans.push(range);
                }
                _ => {}
            }
        }
    }
    for broken_ref in &broken {
        // Shortcut references that resolve nowhere are literal text, never
        // links; explicit (`[text][label]`) and collapsed (`[text][]`)
        // references fail closed like any undeclared target.
        if broken_ref.link_type == LinkType::Shortcut {
            continue;
        }
        let at = line_of(&starts, broken_ref.span.start);
        doc_links.push(DocLink::UndefinedRef {
            line: at,
            col: broken_ref.span.start,
            label: broken_label(text, broken_ref),
        });
    }

    // Relative autolinks (`<./other.md>`) are not CommonMark autolinks, so
    // the parser emits no event for them: scan unscanned lines for `<target>`
    // forms exactly as the retired line scanner did, skipping code spans,
    // link spans, inline HTML tags, and HTML blocks.
    for (index, line) in source_lines.iter().enumerate() {
        let line_no = (index as u32) + 1;
        if suppressed.contains(&line_no) {
            continue;
        }
        let base = starts[index];
        doc_links.extend(scan_bare_autolinks(
            line,
            base,
            line_no,
            &code_spans,
            &link_spans,
            &html_spans,
        ));
    }

    // ATX-only scope is retained: a heading event whose source line is not
    // an ATX heading (blockquote/list markers, setext underlines) counts for
    // nothing, and the slug still derives from the source line.
    for line_no in heading_lines {
        let line = source_lines
            .get((line_no - 1) as usize)
            .copied()
            .unwrap_or("");
        if let Some((level, text)) = heading(line) {
            let base = slug(&text);
            let slug = if base.is_empty() {
                base
            } else {
                let count = slug_counts.entry(base.clone()).or_insert(0);
                let slug = if *count == 0 {
                    base.clone()
                } else {
                    format!("{base}-{}", *count)
                };
                *count += 1;
                slug
            };
            headings.push(Heading {
                level,
                line: line_no,
                slug,
            });
        }
    }

    // Left-to-right per line, matching the retired line scanner order for
    // `skipped_remotes`.
    doc_links.sort_by_key(|link| (link.line(), link.col()));
    check_headings(&headings, &mut outcome);
    let own_slugs: BTreeSet<String> = headings.into_iter().map(|h| h.slug).collect();
    for link in &doc_links {
        match link {
            DocLink::Dest { line, target, .. } => {
                check_target(
                    source_path,
                    *line,
                    target,
                    siblings,
                    &own_slugs,
                    &mut outcome,
                );
            }
            DocLink::UndefinedRef { line, label, .. } => {
                push_finding(
                    &mut outcome,
                    *line,
                    FindingKind::MissingFileTarget,
                    &format!("reference link label has no definition: [{label}]"),
                );
            }
        }
    }
    outcome.findings.sort_by_key(|f| (f.line, f.kind));
    outcome
}

pub(crate) fn push_finding(
    outcome: &mut CheckOutcome,
    line: u32,
    kind: FindingKind,
    message: &str,
) {
    outcome.findings.push(Finding {
        line,
        kind,
        message: message.to_string(),
    });
}

/// One NDJSON finding line on stdout, serialized with serde_json. Field
/// order is part of the stable output shape.
#[derive(Serialize)]
pub(crate) struct FindingLine<'a> {
    pub(crate) path: &'a str,
    pub(crate) line: u32,
    pub(crate) kind: &'static str,
    pub(crate) message: &'a str,
}
/// Stable kebab-case identifier for a [`FindingKind`], used in JSON output.
pub fn kind_id(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::MissingFileTarget => "missing-file-target",
        FindingKind::MissingAnchor => "missing-anchor",
        FindingKind::HeadingHierarchy => "heading-hierarchy",
        FindingKind::MissingCodeFenceLanguage => "missing-code-fence-language",
        FindingKind::UnclosedCodeFence => "unclosed-code-fence",
    }
}

fn usage() -> String {
    "usage: quality_markdown --source WS_PATH=EXEC_PATH [--source ...] [--sibling WS_PATH=EXEC_PATH ...]".into()
}

/// `WS_PATH=EXEC_PATH` mapping behind `--source`/`--sibling`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Mapping {
    ws: String,
    exec: String,
}

/// `argv` tokenizer (frozen legacy contract). `--source`/`--sibling` append in argument
/// order; every value option consumes the next token unconditionally (even
/// a `--`-led token), matching the legacy hand loop. Mapping values validate
/// through [`parse_source_mapping`]/[`parse_sibling_mapping`] at tokenize
/// time; shape failures map back onto the legacy `malformed …` text via
/// [`parse_error`].
#[derive(ClapParser)]
#[command(disable_help_flag = true)]
struct Cli {
    #[arg(long, allow_hyphen_values = true, value_parser = parse_source_mapping)]
    source: Vec<Mapping>,
    #[arg(long, allow_hyphen_values = true, value_parser = parse_sibling_mapping)]
    sibling: Vec<Mapping>,
}

/// Raw `argv` token behind a [`clap::Error`], e.g. `--bogus` or `oops`.
fn invalid_token(error: &clap::Error) -> String {
    match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(token)) => token.clone(),
        Some(ContextValue::Strings(tokens)) => tokens.first().cloned().unwrap_or_default(),
        _ => String::new(),
    }
}

/// Rejected mapping value behind a [`clap::Error`], if the error carries a
/// non-empty one. Empty values (`--source ""`, `--source=`) carry none (or
/// an empty one); the caller resolves those via [`empty_rejection`].
fn rejected_value(error: &clap::Error) -> Option<String> {
    let invalid = error.get(ContextKind::InvalidValue)?;
    let raw = match invalid {
        ContextValue::String(value) => value.clone(),
        ContextValue::Strings(values) => values.first().cloned().unwrap_or_default(),
        _ => String::new(),
    };
    if raw.is_empty() {
        None
    } else {
        Some(raw)
    }
}

/// Empty-value rejection with no carried value: mirrors the legacy
/// left-to-right scan for the first empty shape in `argv`. Attached
/// (`--source=`) was the whole flag word to the legacy loop, so it keeps
/// the unknown-argument form; an explicit empty value token in flag-value
/// position malformed under its flag.
enum EmptyRejection {
    Attached(String),
    Separate(&'static str),
    Neither,
}

fn empty_rejection(args: &[String]) -> EmptyRejection {
    for (index, arg) in args.iter().enumerate() {
        if arg == "--source=" || arg == "--sibling=" {
            return EmptyRejection::Attached(arg.clone());
        }
        if arg.is_empty() && index > 0 {
            if args[index - 1] == "--sibling" {
                return EmptyRejection::Separate("--sibling");
            }
            if args[index - 1] == "--source" {
                return EmptyRejection::Separate("--source");
            }
        }
    }
    EmptyRejection::Neither
}

/// Report the legacy `malformed …` text for `raw` under `flag` by re-running
/// the tokenizing parser (which rejects again), falling back to the raw
/// `clap` first line if it unexpectedly accepts.
fn check_mapping(flag: &str, raw: &str, error: &clap::Error) -> Vec<String> {
    let legacy = if flag == "--sibling" {
        parse_sibling_mapping(raw)
    } else {
        parse_source_mapping(raw)
    };
    match legacy {
        Err(message) => vec![message, usage()],
        Ok(_) => vec![
            error
                .to_string()
                .lines()
                .next()
                .unwrap_or("invalid arguments")
                .to_owned(),
            usage(),
        ],
    }
}

/// Flag whose mapping `raw` rejected, recovered from `argv`: the first
/// occurrence with a known flag before it (a separate value), or the
/// `--flag=` prefix of an attached value. Falls back to `--source`,
/// reachable only for values clap reports that appear in neither form
/// (impossible for real `argv`).
fn rejecting_flag(args: &[String], raw: &str) -> &'static str {
    for (index, arg) in args.iter().enumerate() {
        if arg == raw && index > 0 {
            if args[index - 1] == "--sibling" {
                return "--sibling";
            }
            if args[index - 1] == "--source" {
                return "--source";
            }
        }
    }
    for flag in ["--source", "--sibling"] {
        if args.iter().any(|arg| arg == &format!("{flag}={raw}")) {
            return flag;
        }
    }
    "--source"
}

/// Map `clap` tokenizing failures onto the legacy [`usage`]-routed surface:
/// every failure prints its reason plus the usage line (exit `2`).
/// Reachable kinds: [`ErrorKind::UnknownArgument`], [`ErrorKind::InvalidValue`]
/// (a present flag with no consumable value), and [`ErrorKind::ValueValidation`]
/// (a mapping rejected by [`parse_source_mapping`]/[`parse_sibling_mapping`],
/// the only custom value parsers). No other parser, conflict, or count error
/// can fire.
fn parse_error(error: clap::Error, args: &[String]) -> Vec<String> {
    let token = invalid_token(&error);
    match error.kind() {
        // `clap` strips an attached `=value` from the reported token; the
        // legacy loop echoed the whole `argv` element, so recover it.
        ErrorKind::UnknownArgument => {
            let echoed = args
                .iter()
                .find(|arg| *arg == &token)
                .or_else(|| {
                    args.iter()
                        .find(|arg| arg.starts_with(&format!("{token}=")))
                })
                .map_or(token.clone(), Clone::clone);
            vec![format!("unknown argument: {echoed}"), usage()]
        }
        // The legacy loop names the bare `--flag` here.
        ErrorKind::InvalidValue => {
            let flag = token.split_whitespace().next().unwrap_or(&token);
            vec![format!("missing value for {flag}"), usage()]
        }
        ErrorKind::ValueValidation => {
            // Only the two mapping flags carry custom value parsers, so any
            // rejection is a malformed mapping: resolve its (flag, value)
            // pair and report the legacy `malformed …` text through the
            // same parser the tokenizer wraps. The parser only runs on
            // present values, so the re-check rejects too.
            if let Some(raw) = rejected_value(&error) {
                let flag = rejecting_flag(args, &raw);
                return check_mapping(flag, &raw, &error);
            }
            // Empty values carry no rejected value; resolve them via the
            // legacy-order `argv` scan instead.
            match empty_rejection(args) {
                EmptyRejection::Attached(echoed) => {
                    vec![format!("unknown argument: {echoed}"), usage()]
                }
                EmptyRejection::Separate(flag) => check_mapping(flag, "", &error),
                EmptyRejection::Neither => check_mapping("--source", "", &error),
            }
        }
        _ => vec![
            error
                .to_string()
                .lines()
                .next()
                .unwrap_or("invalid arguments")
                .to_owned(),
            usage(),
        ],
    }
}

fn parse_args(args: &[String]) -> Result<Cli, Vec<String>> {
    Cli::try_parse_from(
        std::iter::once("quality_markdown").chain(args.iter().map(|arg| arg as &str)),
    )
    .map_err(|error| parse_error(error, args))
}

/// Shared `WS_PATH=EXEC_PATH` shape behind [`parse_source_mapping`] and
/// [`parse_sibling_mapping`]: exactly one `=` with non-empty sides, or the
/// legacy `malformed {flag} …` text naming the rejecting flag.
fn parse_mapping(flag: &'static str, raw: &str) -> Result<Mapping, String> {
    match raw.split_once('=') {
        Some((ws, exec)) if !ws.is_empty() && !exec.is_empty() => Ok(Mapping {
            ws: ws.to_string(),
            exec: exec.to_string(),
        }),
        _ => Err(format!("malformed {flag} {raw:?}, want WS_PATH=EXEC_PATH")),
    }
}

/// `clap` value parser for `--source`: rejections already carry
/// the legacy `malformed --source …` text that [`parse_error`] recovers.
fn parse_source_mapping(raw: &str) -> Result<Mapping, String> {
    parse_mapping("--source", raw)
}

/// `clap` value parser for `--sibling`: rejections already carry
/// the legacy `malformed --sibling …` text that [`parse_error`] recovers.
fn parse_sibling_mapping(raw: &str) -> Result<Mapping, String> {
    parse_mapping("--sibling", raw)
}

/// Check workspace sources against a sibling closure (WP3 binary
/// contract). Returns the process exit code: `0` when every source was
/// checked, `2` on bad arguments, unreadable files, or non-UTF-8 input.
pub fn run_cli(
    args: &[String],
    read_file: &dyn Fn(&str) -> Result<Vec<u8>, MarkdownError>,
    print_out: &mut dyn FnMut(&str),
    print_err: &mut dyn FnMut(&str),
) -> i32 {
    let cli = match parse_args(args) {
        Ok(cli) => cli,
        Err(lines) => {
            for line in &lines {
                print_err(line);
            }
            return 2;
        }
    };
    let sources: Vec<(String, String)> = cli
        .source
        .into_iter()
        .map(|mapping| (mapping.ws, mapping.exec))
        .collect();
    let sibling_specs: Vec<(String, String)> = cli
        .sibling
        .into_iter()
        .map(|mapping| (mapping.ws, mapping.exec))
        .collect();

    let mut siblings: BTreeMap<String, String> = BTreeMap::new();
    let mut ordered_ws: Vec<String> = Vec::new();
    for (ws, _) in &sources {
        if !ordered_ws.contains(ws) {
            ordered_ws.push(ws.clone());
        }
    }
    for (ws, exec) in sources.iter().chain(sibling_specs.iter()) {
        if siblings.contains_key(ws) {
            continue;
        }
        let bytes = match read_file(exec) {
            Ok(bytes) => bytes,
            Err(message) => {
                print_err(&format!("cannot read {ws}: {message}"));
                return 2;
            }
        };
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                print_err(&format!("not UTF-8: {ws}"));
                return 2;
            }
        };
        siblings.insert(ws.clone(), text);
    }

    let mut seen_remotes: Vec<String> = Vec::new();
    for ws in &ordered_ws {
        let text = siblings.get(ws).cloned().unwrap_or_default();
        let outcome = check_markdown(ws, &text, &siblings);
        for finding in &outcome.findings {
            let line = FindingLine {
                path: ws.as_str(),
                line: finding.line,
                kind: kind_id(finding.kind),
                message: finding.message.as_str(),
            };
            print_out(
                &serde_json::to_string(&line)
                    .unwrap_or_else(|err| unreachable!("finding line serializes: {err:?}")),
            );
        }
        for remote in &outcome.skipped_remotes {
            if !seen_remotes.iter().any(|seen| seen == remote) {
                seen_remotes.push(remote.clone());
                print_err(&format!("skipped remote target (never fetched): {remote}"));
            }
        }
    }
    0
}
