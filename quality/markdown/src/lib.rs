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
//! Scope notes: inline links (`[text](target)`), images, autolinks, and
//! reference links (`[text][label]` with a `[label]: target` definition,
//! including collapsed `[text][]` and shortcut `[text]` forms when the text
//! matches a definition) are resolved. Email autolinks (`<a@b.c>`) are out of
//! scope and ignored. Inline code spans suppress link detection with an
//! approximate same-length backtick toggle. Heading slugs follow the
//! checker's own rule ([`slug`]): lowercase alphanumerics, `-`/`_` kept,
//! each whitespace character becomes `-`, GitHub-style `-1`/`-2`
//! deduplication for repeat headings.
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

enum RawTarget {
    Inline(String),
    Reference { label: String, explicit: bool },
}

struct PendingLink {
    line: u32,
    target: RawTarget,
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
    let mut pending: Vec<PendingLink> = Vec::new();
    let mut definitions: BTreeMap<String, String> = BTreeMap::new();
    let mut slug_counts: BTreeMap<String, usize> = BTreeMap::new();

    let mut open_fence: Option<(char, usize, u32)> = None;
    for (index, line) in text.lines().enumerate() {
        let line_no = (index as u32) + 1;
        if let Some(marker) = fence_marker(line) {
            match open_fence {
                None => {
                    if marker.2.trim().is_empty() {
                        push_finding(
                            &mut outcome,
                            line_no,
                            FindingKind::MissingCodeFenceLanguage,
                            "fenced code block without a language tag",
                        );
                    }
                    open_fence = Some((marker.0, marker.1, line_no));
                }
                Some((open_char, open_len, _))
                    if marker.0 == open_char
                        && marker.1 >= open_len
                        && marker.2.trim().is_empty() =>
                {
                    open_fence = None;
                }
                Some(_) => {}
            }
            continue;
        }
        if open_fence.is_some() {
            continue;
        }
        if let Some((label, target)) = link_definition(line) {
            definitions.insert(label, target);
            continue;
        }
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
        pending.extend(scan_links(line).into_iter().map(|target| PendingLink {
            line: line_no,
            target,
        }));
    }
    if let Some((_, _, open_line)) = open_fence {
        push_finding(
            &mut outcome,
            open_line,
            FindingKind::UnclosedCodeFence,
            "fenced code block opened here is never closed",
        );
    }

    check_headings(&headings, &mut outcome);
    let own_slugs: BTreeSet<String> = headings.into_iter().map(|h| h.slug).collect();
    for link in &pending {
        check_link(
            source_path,
            link,
            &definitions,
            siblings,
            &own_slugs,
            &mut outcome,
        );
    }
    outcome.findings.sort_by_key(|f| (f.line, f.kind));
    outcome
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

fn check_link(
    source_path: &str,
    link: &PendingLink,
    definitions: &BTreeMap<String, String>,
    siblings: &BTreeMap<String, String>,
    own_slugs: &BTreeSet<String>,
    outcome: &mut CheckOutcome,
) {
    let raw = match &link.target {
        RawTarget::Inline(target) => target.clone(),
        RawTarget::Reference { label, explicit } => match definitions.get(&label.to_lowercase()) {
            Some(target) => target.clone(),
            None => {
                if !explicit {
                    // Bare `[text]` with no definition is literal text, not
                    // a link.
                    return;
                }
                push_finding(
                    outcome,
                    link.line,
                    FindingKind::MissingFileTarget,
                    &format!("reference link label has no definition: [{label}]"),
                );
                return;
            }
        },
    };
    check_target(source_path, link.line, &raw, siblings, own_slugs, outcome);
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
        let Some(content) = siblings.get(&resolved) else {
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
/// the base slug of each heading is trusted outside its own file.
fn slugs_in(content: &str) -> BTreeSet<String> {
    let mut slugs = BTreeSet::new();
    let mut open = false;
    for line in content.lines() {
        if let Some(marker) = fence_marker(line) {
            if !open {
                open = true;
            } else if marker.2.trim().is_empty() {
                open = false;
            }
            continue;
        }
        if open {
            continue;
        }
        if let Some((_, text)) = heading(line) {
            slugs.insert(slug(&text));
        }
    }
    slugs
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

/// Minimal JSON string escaper for finding output: the crate stays
/// dependency-free, so only `"`, `\`, C0 controls, and the standard short
/// escapes are handled; every other character passes through unchanged.
pub fn escape_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
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
            print_out(&format!(
                "{{\"path\":\"{}\",\"line\":{},\"kind\":\"{}\",\"message\":\"{}\"}}",
                escape_json(ws),
                finding.line,
                kind_id(finding.kind),
                escape_json(&finding.message),
            ));
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

/// `[label]: target` link definition: (lowercased label, target).
fn link_definition(line: &str) -> Option<(String, String)> {
    let stripped = line.trim_start();
    if line.len() - stripped.len() > 3 || !stripped.starts_with('[') {
        return None;
    }
    let close = stripped.find("]:")?;
    let label = stripped[1..close].trim().to_lowercase();
    if label.is_empty() {
        return None;
    }
    let after = stripped[close + 2..].trim();
    let target = after.split_whitespace().next().unwrap_or("");
    let target = target
        .strip_prefix('<')
        .and_then(|t| t.strip_suffix('>'))
        .unwrap_or(target);
    if target.is_empty() {
        return None;
    }
    Some((label, target.to_string()))
}

/// Raw link targets on one fence-free line: inline `](target)` forms plus
/// reference usages (`[text][label]`, collapsed `[text][]`, shortcut
/// `[text]`). Images share the same syntax. `<autolinks>` resolve as
/// targets except email forms, which are out of scope. Code spans between
/// matching backtick runs are skipped.
fn scan_links(line: &str) -> Vec<RawTarget> {
    let chars: Vec<char> = line.chars().collect();
    let mut targets = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            i = skip_code_span(&chars, i);
            continue;
        }
        if chars[i] == '<' {
            if let Some(end) = chars[i..].iter().position(|c| *c == '>') {
                let content: String = chars[i + 1..i + end].iter().collect();
                let content = content.trim();
                if !content.is_empty()
                    && !content.contains(char::is_whitespace)
                    && !content.contains('@')
                {
                    targets.push(RawTarget::Inline(content.to_string()));
                }
                i += end + 1;
                continue;
            }
            i += 1;
            continue;
        }
        if chars[i] == '[' || (chars[i] == '!' && chars.get(i + 1) == Some(&'[')) {
            let open = if chars[i] == '!' { i + 1 } else { i };
            match scan_bracket_link(&chars, open) {
                Some((next, target)) => {
                    // A bare `[text]` that matches no definition is literal
                    // text, not a link; resolve it only when the label is
                    // defined by deferring through the Reference form.
                    targets.push(target);
                    i = next;
                    continue;
                }
                None => {
                    i = open + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    targets
}

fn skip_code_span(chars: &[char], start: usize) -> usize {
    let run = chars[start..].iter().take_while(|c| **c == '`').count();
    let mut j = start + run;
    while j < chars.len() {
        if chars[j] == '`' {
            let run2 = chars[j..].iter().take_while(|c| **c == '`').count();
            if run2 == run {
                return j + run2;
            }
            j += run2;
        } else {
            j += 1;
        }
    }
    start + run
}

/// Parse a `[...]` link starting at the `[` at `open`. Returns the index
/// after the link plus its raw target. `[text][label]` and collapsed
/// `[text][]` are explicit references (an undefined label fails closed); a
/// bare `[text]` is a shortcut reference that is literal text unless the
/// label is defined.
fn scan_bracket_link(chars: &[char], open: usize) -> Option<(usize, RawTarget)> {
    let mut depth = 0;
    let mut j = open;
    let mut text_end: Option<usize> = None;
    while j < chars.len() && text_end.is_none() {
        if chars[j] == '[' {
            depth += 1;
        } else if chars[j] == ']' {
            depth -= 1;
            if depth == 0 {
                text_end = Some(j);
            }
        }
        j += 1;
    }
    let text_end = text_end?;
    let text: String = chars[open + 1..text_end].iter().collect();
    let after = text_end + 1;
    if chars.get(after) == Some(&'(') {
        let mut pdepth = 1;
        let mut k = after + 1;
        while k < chars.len() && pdepth > 0 {
            if chars[k] == '(' {
                pdepth += 1;
            } else if chars[k] == ')' {
                pdepth -= 1;
            }
            k += 1;
        }
        if pdepth != 0 {
            return None;
        }
        let raw: String = chars[after + 1..k - 1].iter().collect();
        let target = raw.split_whitespace().next().unwrap_or("").to_string();
        return Some((k, RawTarget::Inline(target)));
    }
    if chars.get(after) == Some(&'[') {
        let mut k = after + 1;
        while k < chars.len() && chars[k] != ']' {
            k += 1;
        }
        if k >= chars.len() {
            return None;
        }
        let label: String = chars[after + 1..k].iter().collect();
        let label = if label.trim().is_empty() { text } else { label };
        return Some((
            k + 1,
            RawTarget::Reference {
                label,
                explicit: true,
            },
        ));
    }
    // Bare `[text]`: a shortcut reference, literal text unless defined.
    Some((
        text_end + 1,
        RawTarget::Reference {
            label: text,
            explicit: false,
        },
    ))
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
    fn cli_json_escapes_message_text() {
        assert_eq!(escape_json("a\"b\\c"), "a\\\"b\\\\c");
        assert_eq!(escape_json("l1\nl2\r\ttab"), "l1\\nl2\\r\\ttab");
        assert_eq!(escape_json("bell\x07"), "bell\\u0007");
        assert_eq!(escape_json("héllo—✓"), "héllo—✓");
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
