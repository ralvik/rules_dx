use std::collections::{BTreeMap, BTreeSet};

use pulldown_cmark::LinkType;

use super::check::{push_finding, CheckOutcome, FindingKind};
use super::frontmatter::slugs_in;

pub(crate) enum DocLink {
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
    pub(crate) fn line(&self) -> u32 {
        match self {
            DocLink::Dest { line, .. } | DocLink::UndefinedRef { line, .. } => *line,
        }
    }

    pub(crate) fn col(&self) -> usize {
        match self {
            DocLink::Dest { col, .. } | DocLink::UndefinedRef { col, .. } => *col,
        }
    }
}

pub(crate) type Span = std::ops::Range<usize>;

pub(crate) struct BrokenRef {
    pub(crate) span: Span,
    pub(crate) link_type: LinkType,
    pub(crate) reference: String,
}

pub(crate) fn broken_label(text: &str, broken_ref: &BrokenRef) -> String {
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

pub(crate) fn check_target(
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

pub fn resolve_target(source: &str, target: &str) -> String {
    fn normalize<I>(segments: I) -> String
    where
        I: IntoIterator<Item = String>,
    {
        let mut parts: Vec<String> = Vec::new();
        for segment in segments {
            match segment.as_str() {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                other => parts.push(other.to_owned()),
            }
        }
        parts.join("/")
    }
    if let Some(stripped) = target.strip_prefix('/') {
        return normalize(stripped.split('/').map(str::to_owned));
    }
    let parent: Vec<String> = match source.rfind('/') {
        Some(index) => source[..index]
            .split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        None => Vec::new(),
    };
    let mut combined = parent;
    combined.extend(target.split('/').map(str::to_owned));
    normalize(combined)
}

pub(crate) fn scan_bare_autolinks(
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

fn overlaps(first: &Span, second: &Span) -> bool {
    first.start.max(second.start) < first.end.min(second.end)
}
