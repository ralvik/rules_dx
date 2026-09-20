use std::collections::BTreeSet;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::check::{push_finding, CheckOutcome, FindingKind};

pub(crate) struct Heading {
    pub(crate) level: usize,
    pub(crate) line: u32,
    pub(crate) slug: String,
}

pub(crate) fn check_headings(headings: &[Heading], outcome: &mut CheckOutcome) {
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

/// Heading slugs in a sibling file, without cross-file `-1` numbering: only
/// the base slug of each heading is trusted outside its own file. Fenced and
/// indented code regions (per [`code_regions`]) contribute no headings.
pub(crate) fn slugs_in(content: &str) -> BTreeSet<String> {
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
pub(crate) fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

/// 1-based line number containing `offset`.
pub(crate) fn line_of(starts: &[usize], offset: usize) -> u32 {
    starts.partition_point(|start| *start <= offset) as u32
}

/// One pulldown-cmark code region (fenced or indented), in 1-based lines
/// covering the opening marker through the closing marker (or end of input
/// when never closed). `fence_info` is the fenced info string, or `None`
/// for indented blocks.
pub(crate) struct CodeRegion {
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) fence_info: Option<String>,
}

/// Code regions from pulldown-cmark block events (`Options::empty()`: no
/// extensions, so tables and strikethrough stay plain paragraphs exactly as
/// the line scanner expects).
pub(crate) fn code_regions(text: &str, starts: &[usize]) -> Vec<CodeRegion> {
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
pub(crate) fn is_closed_fence(source_lines: &[&str], region: &CodeRegion) -> bool {
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

/// Anchor slug via `slug::slugify` over `deunicode` transliteration (issue
///): ASCII `a-z`/`0-9`/`-` only, collapsed and trimmed; non-ASCII
/// headings transliterate instead of stripping.
pub fn slug(text: &str) -> String {
    slug::slugify(deunicode::deunicode(text))
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
pub(crate) fn heading(line: &str) -> Option<(usize, String)> {
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
