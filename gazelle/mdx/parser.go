// Parser extracts the narrow recognized source facts the MDX Gazelle
// extension needs for one-source ownership and strict dependency resolution.
//
// An `.mdx` document interleaves markdown prose, JSX, fenced code, and
// top-level ESM (`import`/`export`) statements. Only the ESM regions
// contribute dependency references: markdown prose (including fenced
// code and JSX expressions) never does, and the pinned `@mdx-js/mdx`
// compiler stays authoritative at execution time.
//
// Recognition is by a narrow chunk parser, never by regular expression:
// ExtractESMRegions walks the document line by line, tracking fenced
// code spans and HTML comments textually, and collects the raw text of
// each ESM chunk; the shared `scan` engine then reads the literal
// specifiers inside those chunks. Generation never compiles a document.
//
// An ESM chunk opens only on an `import`/`export` opener at column zero
// (the pinned compiler proves indented openers inert, including three
// spaces and tabs) following a boundary: the start of the document, a
// blank line, a closing fence line, an ATX heading, a thematic break,
// a setext underline, or another ESM line. An opener directly after a
// paragraph, blockquote, list-item, or single-line HTML line stays prose
// (lazy paragraph continuation in the pinned compiler) and yields no
// edge. A chunk continues across blank lines only while braces are
// unbalanced; any other markdown block closes it. A trailing text line
// after a closed chunk never removes the chunk's edge.
//
// Fail-closed inertness: an unterminated HTML comment makes the whole
// document inert (the pinned compiler fails loudly there, never with an
// edge). An unclosed fenced-code span hides every later opener but keeps
// earlier collected chunks (the pinned compiler still emits the earlier
// module edges). Single-line block HTML immediately before an opener is
// a recognized conservative gap: the adapter treats that opener as
// prose (no edge) even when the compiler keeps it for empty elements;
// documents in that shape add one blank line to enter the recognized
// subset. Unparseable or edge-free documents contribute no imports and
// never fall back to a guessed JavaScript/TypeScript owner.
//
// Recognized ESM syntax (narrow source-only scope; additional forms
// require parser fixtures before they become recognized):
//
//   - `import "name"`, `import x from "name"`, `import {a} from "name"`,
//     `import * as ns from "name"` (dependency on the normalized spec root).
//   - `export {a} from "name"`, `export * from "name"`,
//     `export * as ns from "name"` (dependency on the normalized spec root).
//   - `import("name")` with a literal single- or double-quoted identity.
//   - `require("name")` with a literal single- or double-quoted identity.
//
// Comments (`//`, `/* */`) are inert: tokens that look like imports inside
// them never produce an edge. String, template, and regex literals are inert
// except for the single specifier string in a recognized position.
// Template-literal specifiers (`` import(`name`) ``) are computed and remain
// the manual kept-dependency boundary. Computed `import(x)` / `require(x)`
// produce no edge and no notice.
//
// Specifier normalization: relative (`./`, `../`, `/`) references contribute
// their basename without the final extension (`./helper.js` -> `helper`);
// bare specifiers contribute the full literal (`react`,
// `@mdx-js/mdx`); `node:`-prefixed and builtin identities are included
// and filtered by callers via IsStdLib.
package mdx

import (
	"path"
	"sort"
	"strings"
)

// ParseImports returns the sorted unique normalized import roots for one
// MDX document. Only ESM regions are examined; markdown prose, JSX, and
// fenced code never contribute. Standard-library identities are included;
// callers filter them via IsStdLib.
func ParseImports(content []byte) []string {
	regions := ExtractESMRegions(content)
	if len(regions) == 0 {
		return nil
	}
	set := make(map[string]struct{})
	add := func(spec string) {
		root := normalizeSpec(spec)
		if root == "" {
			return
		}
		set[root] = struct{}{}
	}
	for _, region := range regions {
		scan(region, add)
	}
	var out []string
	for name := range set {
		out = append(out, name)
	}
	sort.Strings(out)
	return out
}

// normalizeSpec maps one literal specifier to its resolution root.
func normalizeSpec(spec string) string {
	spec = strings.TrimSpace(spec)
	if spec == "" {
		return ""
	}
	if strings.HasPrefix(spec, ".") || strings.HasPrefix(spec, "/") {
		// Relative or absolute path: basename without the final extension.
		// `./dir/` (trailing slash) resolves to the directory name; an
		// empty basename contributes nothing.
		trimmed := strings.TrimSuffix(spec, "/")
		base := path.Base(trimmed)
		if base == "" || base == "." || base == "/" {
			return ""
		}
		if idx := strings.LastIndexByte(base, '.'); idx > 0 {
			base = base[:idx]
		}
		return base
	}
	return spec
}

// ExtractESMRegions returns the raw text of every ESM chunk in an MDX
// document, in source order, or nil when there is none. See the package
// documentation for the recognized subset and the fail-closed rules.
func ExtractESMRegions(src []byte) [][]byte {
	lines := splitLines(src)
	var out [][]byte
	var chunk []string
	depth := 0
	// boundary reports whether an opener on the current line may start a
	// chunk: start of document, a blank line, a fence close, a heading,
	// a thematic break, a setext underline, or an ESM continuation.
	boundary := true
	prevText := false
	inFence := false
	fenceChar := byte(0)
	fenceLen := 0
	inComment := false
	flush := func() {
		if len(chunk) > 0 {
			out = append(out, []byte(strings.Join(chunk, "\n")+"\n"))
			chunk = nil
		}
		depth = 0
	}
	for _, raw := range lines {
		line := raw
		if inComment {
			if end := strings.Index(line, "-->"); end >= 0 {
				inComment = false
				line = line[end+3:]
			} else {
				continue
			}
		}
		if !inFence {
			if idx := strings.Index(line, "<!--"); idx >= 0 {
				if end := strings.Index(line[idx+4:], "-->"); end >= 0 {
					line = line[:idx] + line[idx+4+end+3:]
				} else {
					// Unterminated HTML comment: loud failure in the
					// pinned compiler, never an edge. Whole document
					// inert: drop even earlier chunks.
					return nil
				}
			}
		}
		if !inFence {
			if ch, ln, ok := parseFenceOpen(line); ok {
				flush()
				inFence = true
				fenceChar = ch
				fenceLen = ln
				boundary = false
				prevText = false
				continue
			}
		} else {
			if isFenceClose(line, fenceChar, fenceLen) {
				inFence = false
				boundary = true
				prevText = false
				continue
			}
			continue
		}
		if strings.TrimSpace(line) == "" {
			if depth == 0 {
				flush()
			} else {
				chunk = append(chunk, line)
			}
			boundary = true
			prevText = false
			continue
		}
		if isESMOpener(line) {
			if len(chunk) > 0 || boundary {
				chunk = append(chunk, line)
				depth += braceDelta(line)
				boundary = true
				prevText = false
				continue
			}
			// Paragraph continuation: prose, never an edge.
			boundary = false
			prevText = true
			continue
		}
		if len(chunk) > 0 && depth > 0 && isESMContinuation(line) {
			chunk = append(chunk, line)
			depth += braceDelta(line)
			boundary = false
			prevText = false
			continue
		}
		flush()
		boundary = isBoundaryLine(line, prevText)
		prevText = !boundary
	}
	flush()
	if len(out) == 0 {
		return nil
	}
	return out
}

// splitLines splits src after each `\n`, stripping one trailing `\r`
// per line. The final line without a newline is kept.
func splitLines(src []byte) []string {
	if len(src) == 0 {
		return nil
	}
	raw := strings.Split(string(src), "\n")
	if raw[len(raw)-1] == "" {
		raw = raw[:len(raw)-1]
	}
	for i := range raw {
		raw[i] = strings.TrimSuffix(raw[i], "\r")
	}
	return raw
}

// isESMOpener reports whether line starts an ESM statement at column
// zero: `import` or `export` followed by a same-statement character.
// Indented lines are prose or indented code, never ESM.
func isESMOpener(line string) bool {
	if strings.HasPrefix(line, "import") {
		if len(line) == 6 {
			return false
		}
		switch line[6] {
		case ' ', '\t', '"', '\'', '(', '{', '*':
			return true
		}
		return false
	}
	if strings.HasPrefix(line, "export") {
		if len(line) == 6 {
			return false
		}
		switch line[6] {
		case ' ', '\t', '{', '*':
			return true
		}
		return false
	}
	return false
}

// isESMContinuation reports whether a non-opener line continues an open
// (brace-unbalanced) ESM chunk: a closing-brace line or a quoted-specifier
// tail such as `} from "./y";`.
func isESMContinuation(line string) bool {
	trimmed := strings.TrimLeft(line, " \t")
	return strings.HasPrefix(trimmed, "}") || strings.HasPrefix(trimmed, "from ") || strings.HasPrefix(trimmed, "from\t")
}

// isBoundaryLine reports whether a non-blank, non-ESM line still bounds
// the next opener: ATX headings, thematic breaks, and setext underlines
// after text. Every other line (paragraph text, quotes, list items,
// HTML/JSX lines, indented code) absorbs a following opener.
func isBoundaryLine(line string, prevText bool) bool {
	if isATXHeading(line) || isThematicBreak(line) {
		return true
	}
	if prevText && isSetextUnderline(line) {
		return true
	}
	return false
}

// isATXHeading reports `#`-style headings (`# T`, `## T ##`); up to
// three leading spaces are tolerated, four or more are indented code.
func isATXHeading(line string) bool {
	i := 0
	for i < len(line) && line[i] == ' ' && i < 4 {
		i++
	}
	if i >= 4 {
		return false
	}
	hashes := 0
	for i < len(line) && line[i] == '#' {
		hashes++
		i++
	}
	if hashes == 0 || hashes > 6 {
		return false
	}
	if i >= len(line) {
		return true
	}
	return line[i] == ' ' || line[i] == '\t'
}

// isThematicBreak reports `---`, `***`, and `___` breaks (three or more
// of one marker with optional spaces/tabs). A `---` line is always a
// boundary here, including its setext-underline reading.
func isThematicBreak(line string) bool {
	stripped := strings.ReplaceAll(strings.ReplaceAll(line, " ", ""), "\t", "")
	if len(stripped) < 3 {
		return false
	}
	mark := stripped[0]
	if mark != '-' && mark != '*' && mark != '_' {
		return false
	}
	for i := 1; i < len(stripped); i++ {
		if stripped[i] != mark {
			return false
		}
	}
	return true
}

// isSetextUnderline reports `===`/`---` underlines. `---` is covered by
// isThematicBreak; this covers the `===` form, which bounds only after
// a text line (a lone `===` is paragraph text).
func isSetextUnderline(line string) bool {
	stripped := strings.ReplaceAll(strings.ReplaceAll(line, " ", ""), "\t", "")
	if len(stripped) < 1 {
		return false
	}
	for i := 0; i < len(stripped); i++ {
		if stripped[i] != '=' {
			return false
		}
	}
	return true
}

// parseFenceOpen reports a fenced-code opener: up to three leading
// spaces, then three or more backticks or tildes. Info strings are
// allowed; closers are matched by isFenceClose.
func parseFenceOpen(line string) (byte, int, bool) {
	i := 0
	for i < len(line) && line[i] == ' ' {
		i++
	}
	if i > 3 {
		return 0, 0, false
	}
	if i >= len(line) || (line[i] != '`' && line[i] != '~') {
		return 0, 0, false
	}
	ch := line[i]
	n := 0
	for i < len(line) && line[i] == ch {
		n++
		i++
	}
	if n < 3 {
		return 0, 0, false
	}
	return ch, n, true
}

// isFenceClose reports a matching fenced-code closer: up to three
// leading spaces, then at least as many fence characters as opened,
// then only spaces or tabs.
func isFenceClose(line string, ch byte, ln int) bool {
	i := 0
	for i < len(line) && line[i] == ' ' {
		i++
	}
	if i > 3 {
		return false
	}
	n := 0
	for i < len(line) && line[i] == ch {
		n++
		i++
	}
	if n < ln {
		return false
	}
	for i < len(line) {
		if line[i] != ' ' && line[i] != '\t' {
			return false
		}
		i++
	}
	return true
}

// braceDelta counts unbalanced `{` minus `}` on one line, skipping
// single/double-quoted spans and line comments so braces inside
// specifiers and comments never extend a chunk.
func braceDelta(line string) int {
	depth := 0
	i := 0
	for i < len(line) {
		c := line[i]
		if c == '/' && i+1 < len(line) && line[i+1] == '/' {
			break
		}
		if c == '/' && i+1 < len(line) && line[i+1] == '*' {
			j := strings.Index(line[i+2:], "*/")
			if j < 0 {
				break
			}
			i += j + 4
			continue
		}
		if c == '\'' || c == '"' {
			j := i + 1
			for j < len(line) {
				if line[j] == '\\' {
					j += 2
					continue
				}
				if line[j] == c {
					break
				}
				j++
			}
			i = j + 1
			continue
		}
		if c == '{' {
			depth++
		} else if c == '}' {
			depth--
		}
		i++
	}
	return depth
}

// scan walks ESM chunk content in one pass, skipping comments and inert
// literals, and reports each recognized literal specifier via add.
func scan(src []byte, add func(string)) {
	n := len(src)
	i := 0
	for i < n {
		c := src[i]
		// Line comment.
		if c == '/' && i+1 < n && src[i+1] == '/' {
			j := i + 2
			for j < n && src[j] != '\n' {
				j++
			}
			i = j
			continue
		}
		// Block comment.
		if c == '/' && i+1 < n && src[i+1] == '*' {
			j := i + 2
			for j+1 < n && !(src[j] == '*' && src[j+1] == '/') {
				j++
			}
			if j+1 < n {
				i = j + 2
			} else {
				return
			}
			continue
		}
		// Single/double-quoted string outside a recognized position: inert.
		if c == '\'' || c == '"' {
			i = skipQuoted(src, i)
			continue
		}
		// Template literal: inert (computed boundary).
		if c == '`' {
			i = skipTemplate(src, i)
			continue
		}
		// Regex literal heuristic: a `/` that cannot start a comment and is
		// not division. Narrow: when a `/` appears where an expression is
		// expected, skip to the closing unescaped `/`.
		if c == '/' && isRegexStart(src, i) {
			i = skipRegex(src, i)
			continue
		}
		if isIdentStart(c) {
			j := i + 1
			for j < n && isIdentChar(src[j]) {
				j++
			}
			word := string(src[i:j])
			switch word {
			case "import":
				i = parseImport(src, i, j, add)
				continue
			case "export":
				i = parseExport(src, j, add)
				continue
			case "require":
				// A `.require(` method call is not a module load.
				if isPrecededByDot(src, i) {
					i = j
					continue
				}
				i = parseRequire(src, j, add)
				continue
			}
			i = j
			continue
		}
		i++
	}
}

func isIdentStart(c byte) bool {
	return c == '_' || c == '$' ||
		(c >= 'a' && c <= 'z') ||
		(c >= 'A' && c <= 'Z')
}

func isIdentChar(c byte) bool {
	return isIdentStart(c) || (c >= '0' && c <= '9')
}

func isPrecededByDot(src []byte, i int) bool {
	j := i - 1
	for j >= 0 && (src[j] == ' ' || src[j] == '\t' || src[j] == '\n' || src[j] == '\r') {
		j--
	}
	return j >= 0 && src[j] == '.'
}

// isRegexStart reports whether the `/` at i plausibly opens a regex
// literal: the previous significant byte cannot end an expression.
func isRegexStart(src []byte, i int) bool {
	j := i - 1
	for j >= 0 && (src[j] == ' ' || src[j] == '\t' || src[j] == '\n' || src[j] == '\r') {
		j--
	}
	if j < 0 {
		return true
	}
	p := src[j]
	if p == ')' || p == ']' || p == '}' {
		return false
	}
	if isIdentChar(p) || p == '$' || p == '"' || p == '\'' || p == '`' {
		return false
	}
	return true
}

func skipQuoted(src []byte, i int) int {
	quote := src[i]
	j := i + 1
	for j < len(src) {
		if src[j] == '\\' {
			j += 2
			continue
		}
		if src[j] == quote {
			return j + 1
		}
		if src[j] == '\n' {
			return j
		}
		j++
	}
	return len(src)
}

func skipTemplate(src []byte, i int) int {
	j := i + 1
	for j < len(src) {
		if src[j] == '\\' {
			j += 2
			continue
		}
		if src[j] == '`' {
			return j + 1
		}
		if src[j] == '$' && j+1 < len(src) && src[j+1] == '{' {
			depth := 1
			j += 2
			for j < len(src) && depth > 0 {
				if src[j] == '{' {
					depth++
				} else if src[j] == '}' {
					depth--
				} else if src[j] == '\'' || src[j] == '"' {
					j = skipQuoted(src, j)
					continue
				} else if src[j] == '`' {
					j = skipTemplate(src, j)
					continue
				}
				j++
			}
			continue
		}
		j++
	}
	return len(src)
}

func skipRegex(src []byte, i int) int {
	j := i + 1
	inClass := false
	for j < len(src) {
		c := src[j]
		if c == '\\' {
			j += 2
			continue
		}
		if c == '\n' {
			return j
		}
		if c == '[' {
			inClass = true
		} else if c == ']' {
			inClass = false
		} else if c == '/' && !inClass {
			j++
			for j < len(src) && isIdentChar(src[j]) {
				j++
			}
			return j
		}
		j++
	}
	return len(src)
}

// skipTrivia advances past whitespace and comments (`//` and `/* */`).
// An unterminated block comment consumes to EOF.
func skipTrivia(src []byte, pos int) int {
	n := len(src)
	p := pos
	for p < n {
		c := src[p]
		if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
			p++
			continue
		}
		if c == '/' && p+1 < n && src[p+1] == '/' {
			j := p + 2
			for j < n && src[j] != '\n' {
				j++
			}
			p = j
			continue
		}
		if c == '/' && p+1 < n && src[p+1] == '*' {
			j := p + 2
			for j+1 < n && !(src[j] == '*' && src[j+1] == '/') {
				j++
			}
			if j+1 < n {
				p = j + 2
				continue
			}
			return n
		}
		return p
	}
	return p
}

// parseImport handles the `import` keyword at [start, j): side-effect
// imports, named/default/namespace imports with `from`, and literal
// `import("name")` calls. Comments and whitespace between tokens are trivia.
func parseImport(src []byte, start, j int, add func(string)) int {
	n := len(src)
	k := skipTrivia(src, j)
	if k < n && (src[k] == '\'' || src[k] == '"') {
		if spec, next, ok := readQuoted(src, k); ok {
			add(spec)
			return next
		}
		return k + 1
	}
	if k < n && src[k] == '(' {
		m := skipTrivia(src, k+1)
		if m < n && (src[m] == '\'' || src[m] == '"') {
			if spec, next, ok := readQuoted(src, m); ok {
				after := skipTrivia(src, next)
				if after < n && src[after] == ')' {
					add(spec)
					return after + 1
				}
				return next
			}
		}
		return k + 1
	}
	// Scan forward to `from` at depth zero (braces only), then read its
	// specifier. A `;` or newline-free `from`-less clause ends the search.
	depth := 0
	p := k
	for p < n {
		if p+1 < n && src[p] == '/' && (src[p+1] == '/' || src[p+1] == '*') {
			p = skipTrivia(src, p)
			continue
		}
		c := src[p]
		if c == '{' {
			depth++
		} else if c == '}' {
			if depth > 0 {
				depth--
			}
		} else if c == '\'' || c == '"' {
			p = skipQuoted(src, p)
			continue
		} else if c == '`' {
			p = skipTemplate(src, p)
			continue
		} else if c == ';' {
			return p + 1
		} else if depth == 0 && isIdentStart(c) {
			q := p + 1
			for q < n && isIdentChar(src[q]) {
				q++
			}
			if string(src[p:q]) == "from" {
				m := skipTrivia(src, q)
				if m < n && (src[m] == '\'' || src[m] == '"') {
					if spec, next, ok := readQuoted(src, m); ok {
						add(spec)
						return next
					}
				}
				return q
			}
			p = q
			continue
		}
		p++
	}
	return p
}

// parseExport handles `export ... from "name"`; bare exports contribute
// nothing.
func parseExport(src []byte, j int, add func(string)) int {
	n := len(src)
	depth := 0
	p := skipTrivia(src, j)
	for p < n {
		if p+1 < n && src[p] == '/' && (src[p+1] == '/' || src[p+1] == '*') {
			p = skipTrivia(src, p)
			continue
		}
		c := src[p]
		if c == '{' {
			depth++
		} else if c == '}' {
			if depth > 0 {
				depth--
			}
		} else if c == '\'' || c == '"' {
			p = skipQuoted(src, p)
			continue
		} else if c == '`' {
			p = skipTemplate(src, p)
			continue
		} else if c == ';' {
			return p + 1
		} else if depth == 0 && c == '*' {
			// `export * from "name"` or `export * as ns from "name"`.
		} else if depth == 0 && isIdentStart(c) {
			q := p + 1
			for q < n && isIdentChar(src[q]) {
				q++
			}
			if string(src[p:q]) == "from" {
				m := skipTrivia(src, q)
				if m < n && (src[m] == '\'' || src[m] == '"') {
					if spec, next, ok := readQuoted(src, m); ok {
						add(spec)
						return next
					}
				}
				return q
			}
			p = q
			continue
		}
		p++
	}
	return p
}

// parseRequire handles `require("name")` with a literal specifier.
func parseRequire(src []byte, j int, add func(string)) int {
	n := len(src)
	k := skipTrivia(src, j)
	if k >= n || src[k] != '(' {
		return j
	}
	m := skipTrivia(src, k+1)
	if m < n && (src[m] == '\'' || src[m] == '"') {
		if spec, next, ok := readQuoted(src, m); ok {
			after := skipTrivia(src, next)
			if after < n && src[after] == ')' {
				add(spec)
				return after + 1
			}
			return next
		}
	}
	return k + 1
}

// readQuoted reads a single- or double-quoted literal at i and returns
// its unescaped value and the offset past the closing quote.
func readQuoted(src []byte, i int) (string, int, bool) {
	quote := src[i]
	var b strings.Builder
	j := i + 1
	for j < len(src) {
		if src[j] == '\\' && j+1 < len(src) {
			b.WriteByte(src[j+1])
			j += 2
			continue
		}
		if src[j] == quote {
			return b.String(), j + 1, true
		}
		if src[j] == '\n' {
			return "", j, false
		}
		b.WriteByte(src[j])
		j++
	}
	return "", len(src), false
}
