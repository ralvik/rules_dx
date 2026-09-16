//! M04 WP3 repository-owned Markdown link/structure checker.
//!
//! Boundary (frozen in `docs/quality/tool-integrations.md`, O20): this crate
//! parses one Markdown source plus its declared sibling-file closure and
//! reports structured findings. It is distinct from Vale: Vale owns prose
//! policy, this checker owns repository structure (relative link targets,
//! anchors, heading hierarchy, fenced code-block language tags). Remote URLs
//! are recorded as skipped and never fetched. Undeclared link targets fail
//! closed as findings, never as silent passes.
//!
//! Scope notes: inline links (`[text](target)`), images, references
//! (`[text][label]` with a `[label]: target` definition, including collapsed
//! `[text][]` and shortcut `[text]` forms), absolute-URI autolinks, and
//! multi-line links resolve through pulldown-cmark events; relative
//! `<./target>` autolinks resolve through a fallback scan (they are not
//! CommonMark autolinks, so the parser emits no event). A target that names
//! a directory resolves to its declared `README.md` index (`docs/cli/`
//! reads `docs/cli/README.md`); an undeclared index fails closed like any
//! undeclared target. Email autolinks (`<a@b.c>`) are out of scope and
//! ignored. Explicit references with no definition fail closed, keeping the
//! author's label spelling; bare `[text]` with no definition is literal
//! text. Code spans suppress link detection natively, including multi-line
//! spans; a stray backtick is literal text, so a link after it is reported
//! (fail-closed). HTML blocks contribute neither headings nor links. Only
//! ATX headings count (blockquote, list, and setext headings do not), and
//! heading slugs follow the checker's own rule ([`slug`]): lowercase
//! alphanumerics, `-`/`_` kept, each whitespace character becomes `-`,
//! GitHub-style `-1`/`-2` deduplication for repeat headings.
//!
//! Binary contract: [`run_cli`] checks `--source WS_PATH=EXEC_PATH` files
//! against a sibling closure (the union of `--source` and
//! `--sibling WS_PATH=EXEC_PATH` contents) and prints one JSON object per
//! finding on stdout:
//! `{"path": WS, "line": N, "kind": KEBAB, "message": TEXT}`.
//! Exit `0` when every source was checked, with or without findings;
//! unresolved links are findings, never operational failures. Exit `2` on
//! bad arguments, unreadable files, or non-UTF-8 input, with the reason on
//! stderr. Skipped remote targets are reported once each on stderr and never
//! fetched. Duplicate source workspace paths keep the first mapping.

use std::collections::{BTreeMap, BTreeSet};

use pulldown_cmark::{BrokenLink, CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};
use serde::Serialize;

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

struct Heading {
    level: usize,
    line: u32,
    slug: String,
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
                    // Email autolinks are out of scope and ignored.
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

/// One link candidate in document order: a resolved destination or an
/// explicitly broken reference label.
enum DocLink {
    Dest {
        line: u32,
        col: usize,
        target: String,
    },
    UndefinedRef {
        line: u32,
        col: usize,
        label: String,
    },
}

impl DocLink {
    fn line(&self) -> u32 {
        match self {
            DocLink::Dest { line, .. } | DocLink::UndefinedRef { line, .. } => *line,
        }
    }

    fn col(&self) -> usize {
        match self {
            DocLink::Dest { col, .. } | DocLink::UndefinedRef { col, .. } => *col,
        }
    }
}

type Span = std::ops::Range<usize>;

/// An unresolved reference from the broken-link callback: byte span of the
/// source form plus its kind and normalized label.
struct BrokenRef {
    span: Span,
    link_type: LinkType,
    reference: String,
}

/// Display label for an explicitly broken reference, read back off the
/// source span so messages keep the author's spelling: the second bracket
/// group of `[text][label]` (or the inner text of collapsed `[text][]`,
/// whose callback span covers only `[text]`).
fn broken_label(text: &str, broken_ref: &BrokenRef) -> String {
    let span = text.get(broken_ref.span.clone()).unwrap_or("");
    let span = span.strip_prefix('!').unwrap_or(span);
    if broken_ref.link_type == LinkType::Collapsed {
        return span
            .strip_prefix('[')
            .unwrap_or(span)
            .strip_suffix(']')
            .unwrap_or(span)
            .to_string();
    }
    match span.rfind("][") {
        Some(index) => span[index + 2..]
            .strip_suffix(']')
            .unwrap_or(&span[index + 2..])
            .to_string(),
        None => broken_ref.reference.clone(),
    }
}

fn push_finding(outcome: &mut CheckOutcome, line: u32, kind: FindingKind, message: &str) {
    outcome.findings.push(Finding {
        line,
        kind,
        message: message.to_string(),
    });
}

fn check_headings(headings: &[Heading], outcome: &mut CheckOutcome) {
    if headings.is_empty() {
        return;
    }
    let mut h1_lines = headings.iter().filter(|h| h.level == 1).map(|h| h.line);
    match h1_lines.next() {
        None => push_finding(
            outcome,
            headings[0].line,
            FindingKind::HeadingHierarchy,
            "no H1 heading: exactly one H1 is required",
        ),
        Some(_) => {
            for extra in h1_lines {
                push_finding(
                    outcome,
                    extra,
                    FindingKind::HeadingHierarchy,
                    "multiple H1 headings: exactly one H1 is required",
                );
            }
        }
    }
    let mut previous = headings[0].level;
    for heading in headings.iter().skip(1) {
        if heading.level > previous + 1 {
            push_finding(
                outcome,
                heading.line,
                FindingKind::HeadingHierarchy,
                &format!(
                    "skipped heading level: H{} follows H{previous}",
                    heading.level
                ),
            );
        }
        previous = heading.level;
    }
}

fn check_target(
    source_path: &str,
    line: u32,
    raw: &str,
    siblings: &BTreeMap<String, String>,
    own_slugs: &BTreeSet<String>,
    outcome: &mut CheckOutcome,
) {
    let mut target = raw.trim().to_string();
    if target.starts_with('<') && target.ends_with('>') && target.len() >= 2 {
        target = target[1..target.len() - 1].trim().to_string();
    }
    if target.is_empty() {
        push_finding(
            outcome,
            line,
            FindingKind::MissingFileTarget,
            "empty link target fails closed",
        );
        return;
    }
    if has_scheme(&target) {
        if !outcome.skipped_remotes.iter().any(|s| s == &target) {
            outcome.skipped_remotes.push(target);
        }
        return;
    }
    let (file_part, anchor) = match target.find('#') {
        Some(index) => (target[..index].to_string(), target[index + 1..].to_string()),
        None => (target, String::new()),
    };
    let sibling_slugs;
    let active: &BTreeSet<String> = if file_part.is_empty() {
        if anchor.is_empty() {
            return;
        }
        own_slugs
    } else {
        let resolved = resolve_target(source_path, &file_part);
        // A directory target resolves to its declared `README.md` index; a
        // directly declared file still wins. The index must be declared: an
        // undeclared directory fails closed with the directory path named.
        let index = if resolved.is_empty() {
            "README.md".to_owned()
        } else {
            resolved.clone() + "/README.md"
        };
        let content = siblings.get(&resolved).or_else(|| siblings.get(&index));
        let Some(content) = content else {
            push_finding(
                outcome,
                line,
                FindingKind::MissingFileTarget,
                &format!("link target does not resolve to a declared sibling: {resolved}"),
            );
            return;
        };
        if anchor.is_empty() {
            return;
        }
        sibling_slugs = slugs_in(content);
        &sibling_slugs
    };
    if !active.contains(&anchor) {
        push_finding(
            outcome,
            line,
            FindingKind::MissingAnchor,
            &format!("link anchor does not match any heading slug: #{anchor}"),
        );
    }
}

/// Heading slugs in a sibling file, without cross-file `-1` numbering: only
/// the base slug of each heading is trusted outside its own file. Fenced and
/// indented code regions (per [`code_regions`]) contribute no headings.
fn slugs_in(content: &str) -> BTreeSet<String> {
    let starts = line_starts(content);
    let mut suppressed: BTreeSet<u32> = BTreeSet::new();
    for region in &code_regions(content, &starts) {
        for line in region.start_line..=region.end_line {
            suppressed.insert(line);
        }
    }
    let mut slugs = BTreeSet::new();
    for (index, line) in content.lines().enumerate() {
        if suppressed.contains(&((index as u32) + 1)) {
            continue;
        }
        if let Some((_, text)) = heading(line) {
            slugs.insert(slug(&text));
        }
    }
    slugs
}

/// Byte offset where each line starts; `starts[0]` is always `0`.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

/// 1-based line number containing `offset`.
fn line_of(starts: &[usize], offset: usize) -> u32 {
    starts.partition_point(|start| *start <= offset) as u32
}

/// One pulldown-cmark code region (fenced or indented), in 1-based lines
/// covering the opening marker through the closing marker (or end of input
/// when never closed). `fence_info` is the fenced info string, or `None`
/// for indented blocks.
struct CodeRegion {
    start_line: u32,
    end_line: u32,
    fence_info: Option<String>,
}

/// Code regions from pulldown-cmark block events (`Options::empty()`: no
/// extensions, so tables and strikethrough stay plain paragraphs exactly as
/// the line scanner expects).
fn code_regions(text: &str, starts: &[usize]) -> Vec<CodeRegion> {
    let mut regions = Vec::new();
    let mut open: Option<(u32, Option<String>)> = None;
    for (event, range) in Parser::new_ext(text, Options::empty()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Fenced(info) => Some(info.to_string()),
                    CodeBlockKind::Indented => None,
                };
                open = Some((line_of(starts, range.start), info));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((start_line, fence_info)) = open.take() {
                    // The block range ends just past its last byte: the
                    // closing marker line when closed, the last content line
                    // when the fence runs to end of input.
                    let end_line = line_of(starts, range.end.saturating_sub(1)).max(start_line);
                    regions.push(CodeRegion {
                        start_line,
                        end_line,
                        fence_info,
                    });
                }
            }
            _ => {}
        }
    }
    regions
}

/// Whether a fenced region ends with a compatible closing marker: a strictly
/// later line whose marker shares the opener's run character, runs at least
/// as long, and carries no info string (the retired line scanner's rule).
fn is_closed_fence(source_lines: &[&str], region: &CodeRegion) -> bool {
    if region.end_line <= region.start_line {
        return false;
    }
    let open_line = source_lines
        .get((region.start_line - 1) as usize)
        .copied()
        .unwrap_or("");
    let Some((open_char, open_len, _)) = fence_marker(open_line) else {
        return false;
    };
    let close_line = source_lines
        .get((region.end_line - 1) as usize)
        .copied()
        .unwrap_or("");
    match fence_marker(close_line) {
        Some((close_char, close_len, info)) => {
            close_char == open_char && close_len >= open_len && info.trim().is_empty()
        }
        None => false,
    }
}

/// GitHub-style anchor slug defined by this checker, not by an upstream tool.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if c == '-' || c == '_' {
            out.push(c);
        } else if c.is_whitespace() {
            out.push('-');
        }
    }
    out
}

/// `scheme:` prefix with a multi-character scheme (single letters are
/// Windows drive paths, not remote targets).
fn has_scheme(target: &str) -> bool {
    let Some(colon) = target.find(':') else {
        return false;
    };
    let (scheme, _) = target.split_at(colon);
    scheme.len() > 1
        && scheme.chars().enumerate().all(|(i, c)| {
            c.is_ascii_alphabetic()
                || (i > 0 && (c.is_ascii_digit() || c == '+' || c == '-' || c == '.'))
        })
}

/// Resolve `target` against the parent directory of `source`, lexically
/// normalizing `.`/`..`. A leading `/` resolves from the sibling root.
pub fn resolve_target(source: &str, target: &str) -> String {
    if let Some(stripped) = target.strip_prefix('/') {
        return stripped.to_string();
    }
    let mut parts: Vec<&str> = match source.rfind('/') {
        Some(index) => source[..index]
            .split('/')
            .filter(|s| !s.is_empty())
            .collect(),
        None => Vec::new(),
    };
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// One NDJSON finding line on stdout, serialized with serde_json. Field
/// order is part of the stable output shape.
#[derive(Serialize)]
struct FindingLine<'a> {
    path: &'a str,
    line: u32,
    kind: &'static str,
    message: &'a str,
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

fn print_usage(print_err: &mut dyn FnMut(&str)) {
    print_err("usage: quality_markdown --source WS_PATH=EXEC_PATH [--source ...] [--sibling WS_PATH=EXEC_PATH ...]");
}

/// Check workspace sources against a sibling closure (M04 WP3 binary
/// contract). Returns the process exit code: `0` when every source was
/// checked, `2` on bad arguments, unreadable files, or non-UTF-8 input.
pub fn run_cli(
    args: &[String],
    read_file: &dyn Fn(&str) -> Result<Vec<u8>, String>,
    print_out: &mut dyn FnMut(&str),
    print_err: &mut dyn FnMut(&str),
) -> i32 {
    let mut sources: Vec<(String, String)> = Vec::new();
    let mut sibling_specs: Vec<(String, String)> = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        index += 1;
        let value = args.get(index).cloned();
        index += 1;
        let slot = match flag {
            "--source" => &mut sources,
            "--sibling" => &mut sibling_specs,
            _ => {
                print_err(&format!("unknown argument: {flag}"));
                print_usage(print_err);
                return 2;
            }
        };
        let Some(spec) = value else {
            print_err(&format!("missing value for {flag}"));
            print_usage(print_err);
            return 2;
        };
        let Some((ws, exec)) = spec.split_once('=') else {
            print_err(&format!(
                "malformed {flag} {spec:?}, want WS_PATH=EXEC_PATH"
            ));
            print_usage(print_err);
            return 2;
        };
        if ws.is_empty() || exec.is_empty() {
            print_err(&format!(
                "malformed {flag} {spec:?}, want WS_PATH=EXEC_PATH"
            ));
            print_usage(print_err);
            return 2;
        }
        slot.push((ws.to_string(), exec.to_string()));
    }

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
            print_out(&serde_json::to_string(&line).expect("finding line serializes"));
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

/// ` ``` ` or `~~~` fence marker: (run char, run length, info string).
fn fence_marker(line: &str) -> Option<(char, usize, &str)> {
    let stripped = line.trim_start();
    if line.len() - stripped.len() > 3 {
        return None;
    }
    let run_char = stripped.chars().next()?;
    if run_char != '`' && run_char != '~' {
        return None;
    }
    let run_len = stripped.chars().take_while(|c| *c == run_char).count();
    if run_len < 3 {
        return None;
    }
    let info = &stripped[run_len..];
    if run_char == '`' && info.contains('`') {
        return None;
    }
    Some((run_char, run_len, info.trim_end()))
}

/// ATX heading: (level, text with closing hashes stripped).
fn heading(line: &str) -> Option<(usize, String)> {
    let stripped = line.trim_start();
    if line.len() - stripped.len() > 3 {
        return None;
    }
    let level = stripped.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let after = &stripped[level..];
    if !after.is_empty() && !after.starts_with([' ', '\t']) {
        return None;
    }
    let mut text = after.trim().to_string();
    let trailing_hashes = text.len() - text.trim_end_matches('#').len();
    if trailing_hashes > 0 {
        let before = text[..text.len() - trailing_hashes].to_string();
        if before.ends_with([' ', '\t']) {
            text = before.trim_end().to_string();
        }
    }
    Some((level, text))
}

/// Relative `<autolink>` targets on one unscanned line. The parser emits no
/// event for these (only absolute URIs and emails are CommonMark
/// autolinks), so the retired `<...>` rule is kept as a fallback: a `<`
/// closed on the same line whose trimmed content is non-empty with no
/// whitespace and no `@`. Candidates overlapping a code span, a link the
/// parser already emitted, or an inline HTML tag are skipped.
fn scan_bare_autolinks(
    line: &str,
    base: usize,
    line_no: u32,
    code_spans: &[Span],
    link_spans: &[Span],
    html_spans: &[Span],
) -> Vec<DocLink> {
    let bytes = line.as_bytes();
    let mut links = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let Some(length) = bytes[i..].iter().position(|byte| *byte == b'>') else {
            i += 1;
            continue;
        };
        let candidate = base + i..base + i + length + 1;
        i += length + 1;
        let content = line[candidate.start + 1 - base..candidate.end - 1 - base].trim();
        if content.is_empty()
            || content.contains(char::is_whitespace)
            || content.contains('@')
            || code_spans.iter().any(|span| overlaps(span, &candidate))
            || link_spans.iter().any(|span| overlaps(span, &candidate))
            || html_spans.iter().any(|span| overlaps(span, &candidate))
        {
            continue;
        }
        links.push(DocLink::Dest {
            line: line_no,
            col: candidate.start,
            target: content.to_string(),
        });
    }
    links
}

/// Whether two byte spans share at least one byte.
fn overlaps(first: &Span, second: &Span) -> bool {
    first.start.max(second.start) < first.end.min(second.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn siblings(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    fn kinds(outcome: &CheckOutcome) -> Vec<(u32, FindingKind)> {
        outcome.findings.iter().map(|f| (f.line, f.kind)).collect()
    }

    #[test]
    fn clean_document_has_no_findings() {
        let text =
            "# Title\n\nSee [other](other.md) and [section](#title).\n\n```rust\nlet x = 1;\n```\n";
        let outcome = check_markdown(
            "docs/page.md",
            text,
            &siblings(&[("docs/other.md", "# Other\n")]),
        );
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
        assert!(outcome.skipped_remotes.is_empty());
    }

    #[test]
    fn missing_file_target_fails_closed() {
        let outcome = check_markdown(
            "docs/page.md",
            "# Title\n\nSee [gone](gone.md).\n",
            &siblings(&[]),
        );
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingFileTarget)]);
    }

    #[test]
    fn empty_target_fails_closed() {
        let outcome = check_markdown("docs/page.md", "# Title\n\nSee [x]().\n", &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingFileTarget)]);
    }

    #[test]
    fn missing_anchor_is_a_finding() {
        let outcome = check_markdown(
            "docs/page.md",
            "# Title\n\nSee [here](#nope).\n",
            &siblings(&[]),
        );
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingAnchor)]);
    }

    #[test]
    fn sibling_anchor_resolves() {
        let text = "# Title\n\nSee [deep](sub/other.md#part-two).\n";
        let outcome = check_markdown(
            "docs/page.md",
            text,
            &siblings(&[("docs/sub/other.md", "# Part One\n\n## Part Two\n")]),
        );
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn sibling_missing_anchor_is_a_finding() {
        let text = "# Title\n\nSee [deep](sub/other.md#absent).\n";
        let outcome = check_markdown(
            "docs/page.md",
            text,
            &siblings(&[("docs/sub/other.md", "# Part One\n")]),
        );
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingAnchor)]);
    }

    #[test]
    fn multiple_h1_is_a_finding() {
        let outcome = check_markdown("a.md", "# One\n\n# Two\n", &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::HeadingHierarchy)]);
    }

    #[test]
    fn missing_h1_is_a_finding() {
        let outcome = check_markdown("a.md", "## Only\n", &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(1, FindingKind::HeadingHierarchy)]);
    }

    #[test]
    fn skipped_level_is_a_finding() {
        let outcome = check_markdown("a.md", "# T\n\n### Deep\n", &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::HeadingHierarchy)]);
    }

    #[test]
    fn fence_without_language_is_a_finding() {
        let outcome = check_markdown("a.md", "# T\n\n```\ncode\n```\n", &siblings(&[]));
        assert_eq!(
            kinds(&outcome),
            vec![(3, FindingKind::MissingCodeFenceLanguage)]
        );
    }

    #[test]
    fn unclosed_fence_is_a_finding() {
        let outcome = check_markdown("a.md", "# T\n\n```rust\ncode\n", &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::UnclosedCodeFence)]);
    }

    #[test]
    fn fence_closed_by_longer_run_reopens_after() {
        // ```` closes the ```rust block; the trailing ``` opens a new
        // untagged block that runs to end of input.
        let text = "# T\n\n```rust\n````\n```\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert_eq!(
            kinds(&outcome),
            vec![
                (5, FindingKind::MissingCodeFenceLanguage),
                (5, FindingKind::UnclosedCodeFence),
            ]
        );
    }

    #[test]
    fn unclosed_tilde_fence_is_a_finding() {
        let outcome = check_markdown("a.md", "# T\n\n~~~\ncode\n", &siblings(&[]));
        assert_eq!(
            kinds(&outcome),
            vec![
                (3, FindingKind::MissingCodeFenceLanguage),
                (3, FindingKind::UnclosedCodeFence),
            ]
        );
    }

    #[test]
    fn fence_info_with_extra_words_has_language() {
        let text = "# T\n\n```rust foo\ncode\n```\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn unclosed_fence_hides_rest_of_document() {
        let text = "# T\n\n```rust\n[gone](gone.md)\n\n## Late\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::UnclosedCodeFence)]);
    }

    #[test]
    fn indented_code_content_is_suppressed() {
        // Indented code blocks are CommonMark code: links and headings
        // inside are content, never findings. (The retired line scanner
        // treated indented fences as literal text and scanned them.)
        let text = "# T\n\n    [gone](gone.md)\n\n    ## Fake\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn sibling_indented_code_headings_are_ignored() {
        let text = "# T\n\nSee [r](other.md#real).\n";
        let sibling = "# Part\n\n    ## Fake\n\n## Real\n";
        let outcome = check_markdown("a.md", text, &siblings(&[("other.md", sibling)]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
        let bad = "# T\n\nSee [f](other.md#fake).\n";
        let outcome = check_markdown("a.md", bad, &siblings(&[("other.md", sibling)]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingAnchor)]);
    }

    #[test]
    fn remote_targets_are_skipped_never_fetched() {
        let text = "# T\n\nSee [web](https://example.com/page#frag).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty());
        assert_eq!(
            outcome.skipped_remotes,
            vec!["https://example.com/page#frag".to_string()]
        );
    }

    #[test]
    fn email_autolink_is_out_of_scope() {
        let text = "# T\n\nWrite <dev@example.com>.\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
        assert!(outcome.skipped_remotes.is_empty());
    }

    #[test]
    fn links_inside_fences_and_code_spans_are_ignored() {
        let text = "# T\n\n```text\n[gone](gone.md)\n```\n\n`[gone](gone.md)`\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn duplicate_headings_get_numbered_slugs() {
        let text = "# T\n\n## Repeat\n\n## Repeat\n\nSee [second](#repeat-1).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn reference_link_resolves_through_definition() {
        let text = "# T\n\nSee [other][ref].\n\n[ref]: other.md\n";
        let outcome = check_markdown(
            "docs/page.md",
            text,
            &siblings(&[("docs/other.md", "# Other\n")]),
        );
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn undefined_reference_label_fails_closed() {
        let text = "# T\n\nSee [other][missing].\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingFileTarget)]);
        assert_eq!(
            outcome.findings[0].message,
            "reference link label has no definition: [missing]"
        );
    }

    #[test]
    fn undefined_collapsed_reference_fails_closed() {
        let text = "# T\n\nSee [gone][].\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingFileTarget)]);
        assert_eq!(
            outcome.findings[0].message,
            "reference link label has no definition: [gone]"
        );
    }

    #[test]
    fn shortcut_reference_resolves_through_definition() {
        let text = "# T\n\nSee [other].\n\n[other]: other.md\n";
        let outcome = check_markdown(
            "docs/page.md",
            text,
            &siblings(&[("docs/other.md", "# Other\n")]),
        );
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn image_with_undefined_reference_fails_closed() {
        let text = "# T\n\nSee ![alt][missing].\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingFileTarget)]);
    }

    #[test]
    fn uri_autolink_is_skipped_never_fetched() {
        let text = "# T\n\nSee <https://example.com/page#frag>.\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
        assert_eq!(
            outcome.skipped_remotes,
            vec!["https://example.com/page#frag".to_string()]
        );
    }

    #[test]
    fn multiline_link_target_resolves() {
        let text = "# T\n\nSee [multi\nline](other.md).\n";
        let outcome = check_markdown(
            "docs/page.md",
            text,
            &siblings(&[("docs/other.md", "# Other\n")]),
        );
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn html_block_content_is_ignored() {
        // Headings and links inside an HTML block are markup content, never
        // findings (CommonMark renders them verbatim). No blank line: a
        // blank line would end the block and re-expose the link.
        let text = "# T\n\n<div>\n# Fake\n[gone](gone.md)\n</div>\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn inline_html_leaves_line_links_visible() {
        // Inline HTML is not an HTML block: the link still resolves (and the
        // tag itself is markup, never an autolink target).
        let text = "# T\n\nPress <kbd>Ctrl</kbd> plus [gone](gone.md).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert_eq!(kinds(&outcome), vec![(3, FindingKind::MissingFileTarget)]);
        assert!(outcome.findings[0].message.contains("gone.md"));
    }

    #[test]
    fn blockquote_heading_is_not_a_heading() {
        // ATX-only scope: `#` under a blockquote marker is not a heading.
        let outcome = check_markdown("a.md", "> # Quoted\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn bare_brackets_without_definition_are_literal_text() {
        // `[text]` with no definition is literal text, not a link: silent.
        // This documents the shortcut-reference approximation.
        let text = "# T\n\nA [bracket] aside.\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn resolve_target_handles_relative_and_root_paths() {
        assert_eq!(resolve_target("docs/a/b.md", "../c.md"), "docs/c.md");
        assert_eq!(resolve_target("docs/b.md", "/x/y.md"), "x/y.md");
        assert_eq!(resolve_target("b.md", "./c.md"), "c.md");
    }

    #[test]
    fn tilde_run_inside_backtick_fence_is_content() {
        let text = "# T\n\n```rust\n~~~\n```\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn empty_h1_counts_as_the_single_h1() {
        let outcome = check_markdown("a.md", "#\n\nText.\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn document_without_headings_has_no_hierarchy_finding() {
        let text = "See [here](other.md).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[("other.md", "x")]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn angle_bracket_target_strips() {
        let text = "# T\n\nSee [here](<other.md>).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[("other.md", "# O\n")]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn autolink_target_resolves() {
        let text = "# T\n\nSee <./other.md>.\n";
        let outcome = check_markdown("a.md", text, &siblings(&[("other.md", "# O\n")]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn directory_link_resolves_to_declared_readme_index() {
        let text = "# T\n\nSee the [CLI](../cli/) guide.\n";
        let outcome = check_markdown(
            "docs/architecture/notes.md",
            text,
            &siblings(&[("docs/cli/README.md", "# CLI\n")]),
        );
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn directory_link_anchor_checks_index_slugs() {
        let good = "# T\n\nSee [setup](../cli/#setup).\n";
        let outcome = check_markdown(
            "docs/architecture/notes.md",
            good,
            &siblings(&[("docs/cli/README.md", "# CLI\n\n## Setup\n")]),
        );
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
        let bad = "# T\n\nSee [setup](../cli/#missing).\n";
        let outcome = check_markdown(
            "docs/architecture/notes.md",
            bad,
            &siblings(&[("docs/cli/README.md", "# CLI\n\n## Setup\n")]),
        );
        assert_eq!(
            kinds(&outcome),
            vec![(3, FindingKind::MissingAnchor)],
            "{:?}",
            outcome.findings
        );
    }

    #[test]
    fn directory_link_without_declared_index_fails_closed() {
        let text = "# T\n\nSee the [CLI](../cli/) guide.\n";
        let outcome = check_markdown("docs/architecture/notes.md", text, &siblings(&[]));
        assert_eq!(
            kinds(&outcome),
            vec![(3, FindingKind::MissingFileTarget)],
            "{:?}",
            outcome.findings
        );
    }

    #[test]
    fn declared_file_wins_over_directory_index() {
        let text = "# T\n\nSee [cli](cli).\n";
        let outcome = check_markdown(
            "notes.md",
            text,
            &siblings(&[("cli", "# Not a dir\n"), ("cli/README.md", "# Index\n")]),
        );
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn self_directory_link_resolves_to_root_readme_index() {
        // A target normalizing to the source's own directory resolves to
        // its declared `README.md` index (empty resolution branch).
        let text = "# T\n\nSee the [index](./) page.\n";
        let outcome = check_markdown("notes.md", text, &siblings(&[("README.md", "# Root\n")]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn multiline_code_span_hides_autolink() {
        // A code span opened on one line closes on the next; the `<pkg>`
        // inside is span content, not an autolink (doc-ir shape).
        let text = "# T\n\nRun (`dump <pkg> [-o out]\n[-f]`, more).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn span_closer_misread_as_opener_is_fixed_across_lines() {
        // The closing backtick of a multi-line span must not reopen one
        // (M01 report shape): `<short_path>` stays span content.
        let text = "# T\n\nUnset under `bazel\ntest`. Resolve `$WS/<short_path>` here.\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn span_carry_resets_on_blank_line() {
        // A stray backtick is literal text under CommonMark, never a span
        // opener: the link it precedes is reported (fail-closed), while a
        // span still cannot hide links past its paragraph.
        let text = "# T\n\nStray ` opener hides [gone](gone.md).\n\nSee <also-gone.md>.\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert_eq!(
            kinds(&outcome),
            vec![
                (3, FindingKind::MissingFileTarget),
                (5, FindingKind::MissingFileTarget),
            ],
            "{:?}",
            outcome.findings
        );
    }

    #[test]
    fn bare_hash_anchor_links_to_top() {
        let outcome = check_markdown("a.md", "# T\n\nBack to [top](#).\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn sibling_fence_content_headings_are_ignored() {
        let text = "# T\n\nSee [r](other.md#real).\n";
        let sibling = "# Part\n\n```text\n## Fake\n```\n\n## Real\n";
        let outcome = check_markdown("a.md", text, &siblings(&[("other.md", sibling)]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn slug_keeps_dashes_and_underscores() {
        let text = "# T\n\n## well-known_name\n\nSee [s](#well-known_name).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn scheme_with_digit_is_remote() {
        let text = "# T\n\nSee [bucket](s3://bucket/key).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
        assert_eq!(outcome.skipped_remotes, vec!["s3://bucket/key".to_string()]);
    }

    #[test]
    fn indented_fence_and_heading_are_literal_text() {
        let text = "# T\n\n    ```\n    code\n    ```\n\n    ## Not a heading\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn backtick_in_info_string_is_not_a_fence() {
        let outcome = check_markdown("a.md", "# T\n\n```a`b\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn hash_without_space_is_not_a_heading() {
        let outcome = check_markdown("a.md", "# T\n\n#Nope\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn closing_hashes_strip_from_heading_text() {
        let text = "# T\n\n## Title ##\n\nSee [t](#title).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn hash_inside_heading_text_is_kept() {
        let text = "# T\n\n## C# basics\n\nSee [c](#c-basics).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn empty_definition_label_is_not_a_definition() {
        let outcome = check_markdown("a.md", "# T\n\n[]: other.md\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn definition_without_target_is_not_a_definition() {
        let outcome = check_markdown("a.md", "# T\n\n[ref]:\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn autolink_without_close_is_literal_text() {
        let outcome = check_markdown("a.md", "# T\n\na < b\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn unclosed_bracket_is_literal_text() {
        let outcome = check_markdown("a.md", "# T\n\nSee [unclosed here.\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn nested_parens_in_target_resolve() {
        let text = "# T\n\nSee [x](sub(1).md).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[("sub(1).md", "# S\n")]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn unclosed_paren_target_is_literal_text() {
        let outcome = check_markdown("a.md", "# T\n\nSee [x](other.md\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn unclosed_reference_label_is_literal_text() {
        let outcome = check_markdown("a.md", "# T\n\nSee [x][label\n", &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    #[test]
    fn mismatched_code_span_run_skips_gracefully() {
        let text = "# T\n\n``code` and [ok](#t).\n";
        let outcome = check_markdown("a.md", text, &siblings(&[]));
        assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
    }

    fn cli_files(files: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
        files
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.as_bytes().to_vec()))
            .collect()
    }

    fn run_harness(
        files: &BTreeMap<String, Vec<u8>>,
        args: &[&str],
    ) -> (i32, Vec<String>, Vec<String>) {
        let owned_args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_cli(
            &owned_args,
            &|path| {
                files
                    .get(path)
                    .cloned()
                    .ok_or_else(|| "missing fixture".to_string())
            },
            &mut |line| out.push(line.to_string()),
            &mut |line| err.push(line.to_string()),
        );
        (code, out, err)
    }

    #[test]
    fn cli_clean_sources_emit_nothing() {
        let files = cli_files(&[
            ("/exec/page.md", "# T\n\nSee [o](other.md#o).\n"),
            ("/exec/other.md", "# O\n"),
        ]);
        let (code, out, err) = run_harness(
            &files,
            &[
                "--source",
                "page.md=/exec/page.md",
                "--sibling",
                "other.md=/exec/other.md",
            ],
        );
        assert_eq!(code, 0);
        assert!(out.is_empty(), "{out:?}");
        assert!(err.is_empty(), "{err:?}");
    }

    #[test]
    fn cli_sources_resolve_each_other_without_sibling_flags() {
        let files = cli_files(&[
            ("/exec/a.md", "# A\n\nSee [b](b.md).\n"),
            ("/exec/b.md", "# B\n"),
        ]);
        let (code, out, _) = run_harness(
            &files,
            &["--source", "a.md=/exec/a.md", "--source", "b.md=/exec/b.md"],
        );
        assert_eq!(code, 0);
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn cli_findings_are_json_lines_and_exit_zero() {
        let files = cli_files(&[("/exec/page.md", "# T\n\nSee [gone](gone.md).\n")]);
        let (code, out, err) = run_harness(&files, &["--source", "page.md=/exec/page.md"]);
        assert_eq!(code, 0);
        assert_eq!(
            out,
            vec![
                "{\"path\":\"page.md\",\"line\":3,\"kind\":\"missing-file-target\",\"message\":\"link target does not resolve to a declared sibling: gone.md\"}"
                    .to_string()
            ]
        );
        assert!(err.is_empty(), "{err:?}");
    }

    #[test]
    fn cli_findings_serialize_as_stable_json_lines() {
        // Byte shape (field order, escaping, unicode passthrough) is the
        // adapter's NDJSON contract: serde_json agrees with the retired hand
        // escaper except 0x08/0x0c, which serde_json writes as \b/\f.
        let line = FindingLine {
            path: "p.md",
            line: 3,
            kind: "missing-file-target",
            message: "a\"b\\c\né✓",
        };
        assert_eq!(
            serde_json::to_string(&line).expect("serializes"),
            "{\"path\":\"p.md\",\"line\":3,\"kind\":\"missing-file-target\",\"message\":\"a\\\"b\\\\c\\né✓\"}"
        );
        assert_eq!(
            serde_json::to_string(&"bell\x07").expect("serializes"),
            "\"bell\\u0007\""
        );
    }

    #[test]
    fn cli_kind_ids_are_stable() {
        assert_eq!(
            kind_id(FindingKind::UnclosedCodeFence),
            "unclosed-code-fence"
        );
        assert_eq!(kind_id(FindingKind::MissingAnchor), "missing-anchor");
        assert_eq!(kind_id(FindingKind::HeadingHierarchy), "heading-hierarchy");
        assert_eq!(
            kind_id(FindingKind::MissingCodeFenceLanguage),
            "missing-code-fence-language"
        );
        assert_eq!(
            kind_id(FindingKind::MissingFileTarget),
            "missing-file-target"
        );
    }

    #[test]
    fn cli_remotes_are_noted_once_on_stderr() {
        let files = cli_files(&[(
            "/exec/page.md",
            "# T\n\nSee [a](https://example.com/x) and [b](https://example.com/x).\n",
        )]);
        let (code, out, err) = run_harness(&files, &["--source", "page.md=/exec/page.md"]);
        assert_eq!(code, 0);
        assert!(out.is_empty(), "{out:?}");
        assert_eq!(
            err,
            vec!["skipped remote target (never fetched): https://example.com/x".to_string()]
        );
    }

    #[test]
    fn cli_duplicate_source_keeps_first_mapping() {
        let files = cli_files(&[
            ("/exec/first.md", "# First\n"),
            ("/exec/second.md", "# Second\n"),
        ]);
        let (code, out, _) = run_harness(
            &files,
            &[
                "--source",
                "page.md=/exec/first.md",
                "--source",
                "page.md=/exec/second.md",
            ],
        );
        assert_eq!(code, 0);
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn cli_no_sources_is_a_no_op() {
        let files = cli_files(&[]);
        let (code, out, err) = run_harness(&files, &[]);
        assert_eq!(code, 0);
        assert!(out.is_empty());
        assert!(err.is_empty());
    }

    #[test]
    fn cli_unknown_argument_fails() {
        let files = cli_files(&[]);
        let (code, out, err) = run_harness(&files, &["--bogus", "x"]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert_eq!(err.len(), 2, "{err:?}");
        assert!(err[0].contains("unknown argument: --bogus"), "{err:?}");
    }

    #[test]
    fn cli_missing_value_fails() {
        let files = cli_files(&[]);
        let (code, _, err) = run_harness(&files, &["--source"]);
        assert_eq!(code, 2);
        assert!(err[0].contains("missing value for --source"), "{err:?}");
    }

    #[test]
    fn cli_malformed_mapping_fails() {
        let files = cli_files(&[]);
        for spec in ["no-equals", "=no-ws", "no-exec="] {
            let (code, _, err) = run_harness(&files, &["--source", spec]);
            assert_eq!(code, 2, "{spec}");
            assert!(err[0].contains("malformed --source"), "{err:?}");
        }
    }

    #[test]
    fn cli_unreadable_file_fails_closed() {
        let files = cli_files(&[]);
        let (code, out, err) = run_harness(&files, &["--source", "page.md=/exec/page.md"]);
        assert_eq!(code, 2);
        assert!(out.is_empty());
        assert_eq!(
            err,
            vec!["cannot read page.md: missing fixture".to_string()]
        );
    }

    #[test]
    fn cli_non_utf8_reports_path() {
        let mut files = cli_files(&[("/exec/messy.md", "# T\n\nSee [gone](gone.md).\n")]);
        files.insert("/exec/bad.md".to_string(), vec![0xff, 0xfe]);
        let (code, out, err) = run_harness(
            &files,
            &[
                "--source",
                "messy.md=/exec/messy.md",
                "--source",
                "bad.md=/exec/bad.md",
            ],
        );
        assert_eq!(code, 2);
        assert!(out.is_empty(), "{out:?}");
        assert_eq!(err, vec!["not UTF-8: bad.md".to_string()]);
    }
}
