// Parser extracts the narrow recognized source facts the Python Gazelle
// extension needs for one-source ownership and strict dependency resolution.
//
// Recognized syntax (M14 narrow source-only scope; additional forms require
// parser fixtures before they become recognized):
//
//   - `import a`, `import a.b`, `import a as b`, comma-separated lists.
//   - `from a import b`, `from a.b import c, d`, parenthesized lists,
//     `as` aliases, `*` imports (dependency on the source module only).
//   - `importlib.import_module("a")` and `importlib.import_module('a.b')`
//     with a literal string identity (dependency on the root).
//
// Comments, string and character literals are inert: tokens that look like
// imports inside them never produce an edge. Relative imports (`from .`,
// `from .foo`, leading-dot `import` is a syntax error) produce no edge:
// same-package relative references stay inside the one-source owner set and
// never become a Bazel label. Every other literal identity contributes its
// root component (`a.b` -> `a`).
package python

import (
	"sort"
	"strings"
)

// ParseImports returns the sorted unique top-level import roots for one
// Python source file. Relative references produce no entry. Standard-library
// identities are included; callers filter them via IsStdLib.
func ParseImports(content []byte) []string {
	blanked, dynamic := blankAndCollect(string(content))
	set := make(map[string]struct{})
	add := func(root string) {
		root = strings.TrimSpace(root)
		if idx := strings.IndexByte(root, '.'); idx >= 0 {
			root = strings.TrimSpace(root[:idx])
		}
		if root == "" || strings.HasPrefix(root, ".") {
			return
		}
		set[root] = struct{}{}
	}
	for _, d := range dynamic {
		add(d)
	}
	collectStatic(blanked, add)
	var out []string
	for name := range set {
		out = append(out, name)
	}
	sort.Strings(out)
	return out
}

// blankAndCollect blanks comments and string literals with spaces (newlines
// preserved) and collects literal import_module identities. The returned
// blanked source contains no string contents; the dynamic list holds the
// raw literal roots (`a.b` full form, filtered later by the caller).
func blankAndCollect(src string) (string, []string) {
	out := []byte(src)
	n := len(out)
	var dynamic []string
	blank := func(from, to int) {
		for k := from; k < to && k < n; k++ {
			if out[k] != '\n' {
				out[k] = ' '
			}
		}
	}
	i := 0
	for i < n {
		c := out[i]
		if c == '#' {
			j := i + 1
			for j < n && out[j] != '\n' {
				j++
			}
			blank(i, j)
			i = j
			continue
		}
		if c == '\'' || c == '"' {
			// Triple-quoted string takes precedence over single.
			if i+2 < n && out[i+1] == c && out[i+2] == c {
				end := findTripleEnd(out, i+3, c)
				blank(i, end)
				i = end
				continue
			}
			end := findSingleEnd(out, i+1, c)
			blank(i, end)
			i = end
			continue
		}
		if isIdentStart(c) {
			j := i + 1
			for j < n && isIdentChar(out[j]) {
				j++
			}
			word := string(out[i:j])
			if word == "import_module" {
				if root, end, ok := tryImportModule(out, j); ok {
					dynamic = append(dynamic, root)
					blank(i, end)
					i = end
					continue
				}
			}
			i = j
			continue
		}
		i++
	}
	return string(out), dynamic
}

// tryImportModule parses after an `import_module` identifier: optional
// whitespace, `(`, optional whitespace, a single- or triple-quoted literal,
// optional whitespace, `)`. It reports the literal body and the end offset
// just past the closing quote (not past `)` so the blanker preserves
// statement structure). Computed arguments return not-ok.
func tryImportModule(out []byte, pos int) (string, int, bool) {
	n := len(out)
	p := skipInline(out, pos)
	if p >= n || out[p] != '(' {
		return "", 0, false
	}
	p++
	p = skipAll(out, p)
	if p >= n || (out[p] != '\'' && out[p] != '"') {
		return "", 0, false
	}
	quote := out[p]
	if p+2 < n && out[p+1] == quote && out[p+2] == quote {
		end := findTripleEnd(out, p+3, quote)
		if end < 3 || out[end-3] != quote || out[end-2] != quote || out[end-1] != quote {
			// Unterminated literal: leave it inert rather than
			// inferring an edge from trailing garbage.
			return "", 0, false
		}
		body := string(out[p+3 : end-3])
		return strings.TrimSpace(body), end, true
	}
	end := findSingleEnd(out, p+1, quote)
	if end < 1 || out[end-1] != quote {
		// Unterminated literal: leave it inert rather than inferring
		// an edge from the rest of the line.
		return "", 0, false
	}
	body := string(out[p+1 : end-1])
	return strings.TrimSpace(body), end, true
}

// findTripleEnd returns the offset just past the closing triple delimiter,
// or len(out) when unterminated. Backslash escapes are honored uniformly.
func findTripleEnd(out []byte, pos int, quote byte) int {
	n := len(out)
	k := pos
	for k < n {
		if out[k] == '\\' {
			k += 2
			continue
		}
		if out[k] == quote && k+2 < n && out[k+1] == quote && out[k+2] == quote {
			return k + 3
		}
		k++
	}
	return n
}

// findSingleEnd returns the offset just past the closing quote, the end of
// the line for an unterminated literal, or len(out). Backslash escapes are
// honored uniformly.
func findSingleEnd(out []byte, pos int, quote byte) int {
	n := len(out)
	k := pos
	for k < n {
		if out[k] == '\\' {
			k += 2
			continue
		}
		if out[k] == quote {
			return k + 1
		}
		if out[k] == '\n' {
			return k
		}
		k++
	}
	return n
}

// collectStatic parses `import` and `from` statements from blanked source
// (no comments or string contents) and reports each absolute root via add.
func collectStatic(s string, add func(string)) {
	out := []byte(s)
	n := len(out)
	i := 0
	for i < n {
		c := out[i]
		if !isIdentStart(c) {
			i++
			continue
		}
		j := i + 1
		for j < n && isIdentChar(out[j]) {
			j++
		}
		switch string(out[i:j]) {
		case "import":
			i = parseImportList(out, j, add)
		case "from":
			i = parseFrom(out, j, add)
		default:
			i = j
		}
	}
}

// parseImportList parses after an `import` keyword: comma-separated dotted
// names with optional `as` aliases until newline, `;`, or end of input.
// Stray punctuation (plain imports never use parentheses) is skipped so
// scanning always progresses. It reports each root and returns the offset
// to resume scanning from.
func parseImportList(out []byte, pos int, add func(string)) int {
	n := len(out)
	p := pos
	for p < n {
		p = skipInline(out, p)
		if p >= n {
			return p
		}
		switch out[p] {
		case '\n':
			return p + 1
		case ';':
			return p + 1
		case ',':
			p++
			continue
		case '\\':
			// Line continuation: consume the backslash and all following
			// whitespace including newlines, then continue the list.
			p = skipAll(out, p+1)
			continue
		case '.':
			// Leading-dot plain imports are syntax errors; skip the dots
			// so scanning always progresses.
			p++
			continue
		}
		if !isIdentStart(out[p]) {
			// Digits and other punctuation never start a module; skip one
			// byte so unterminated or exotic statements terminate.
			p++
			continue
		}
		root, next := scanDotted(out, p)
		add(root)
		p = skipInline(out, next)
		if hasWord(out, p, "as") {
			p = skipIdent(out, p+2)
			p = skipInline(out, p)
			if p < n && isIdentStart(out[p]) {
				p = skipIdent(out, p)
			}
		}
	}
	return p
}

// parseFrom parses after a `from` keyword: a dotted module (possibly
// relative), the `import` keyword, then imported names which never become
// dependencies. Only the source module contributes a root. Malformed tails
// advance at least one byte so scanning always progresses.
func parseFrom(out []byte, pos int, add func(string)) int {
	n := len(out)
	p := skipInline(out, pos)
	dots := 0
	for p < n && out[p] == '.' {
		dots++
		p++
	}
	p = skipInline(out, p)
	module := ""
	if p < n && isIdentStart(out[p]) {
		var next int
		module, next = scanDotted(out, p)
		p = next
	}
	if dots == 0 && module != "" {
		add(module)
	}
	p = skipInline(out, p)
	// A backslash continuation may separate the module from `import`.
	if p < n && out[p] == '\\' {
		p = skipAll(out, p+1)
	}
	if !hasWord(out, p, "import") {
		// Not a from-import (e.g. `from` used as an identifier tail in
		// blanked exotic code); resume after the module so the `import`
		// keyword later on is still found when it opens its own statement.
		if module == "" {
			if p < n && out[p] != '\n' {
				return p + 1
			}
			return p
		}
		return p
	}
	p += len("import")
	p = skipInline(out, p)
	if p < n && out[p] == '\\' {
		p = skipAll(out, p+1)
	}
	if p < n && out[p] == '(' {
		// Parenthesized name list may span lines; consume to the matching
		// close paren so inner names are never mistaken for module imports.
		// Blanked source holds no strings, so the first close paren ends
		// the list; an unclosed list runs to the end of input.
		for p < n && out[p] != ')' {
			p++
		}
		if p < n {
			return p + 1
		}
		return p
	}
	// Bare name list until newline or `;`; names never become dependencies.
	for p < n && out[p] != '\n' && out[p] != ';' {
		p++
	}
	if p < n {
		return p + 1
	}
	return p
}

// scanDotted scans `ident(.ident)*` at pos and reports the full dotted form
// and the offset just past it. Inline whitespace around the dots is part of
// the form (Python accepts `import a . b`); callers take the root component.
func scanDotted(out []byte, pos int) (string, int) {
	n := len(out)
	p := skipIdent(out, pos)
	for {
		q := skipInline(out, p)
		if q < n && out[q] == '.' {
			r := skipInline(out, q+1)
			if r < n && isIdentStart(out[r]) {
				p = skipIdent(out, r)
				continue
			}
		}
		return string(out[pos:p]), p
	}
}

// hasWord reports whether word starts at pos with a non-identifier boundary
// on both sides. The caller guarantees the leading boundary by only calling
// after whitespace or delimiters.
func hasWord(out []byte, pos int, word string) bool {
	if pos+len(word) > len(out) {
		return false
	}
	if string(out[pos:pos+len(word)]) != word {
		return false
	}
	after := pos + len(word)
	return after >= len(out) || !isIdentChar(out[after])
}

// skipIdent advances past one identifier starting at pos.
func skipIdent(out []byte, pos int) int {
	n := len(out)
	p := pos
	for p < n && isIdentChar(out[p]) {
		p++
	}
	return p
}

// skipInline advances past spaces, tabs, and carriage returns (never
// newlines or semicolons, which terminate import statements).
func skipInline(out []byte, pos int) int {
	for pos < len(out) && (out[pos] == ' ' || out[pos] == '\t' || out[pos] == '\r') {
		pos++
	}
	return pos
}

// skipAll advances past all whitespace including newlines and semicolons,
// used only for backslash continuations.
func skipAll(out []byte, pos int) int {
	for pos < len(out) && (out[pos] == ' ' || out[pos] == '\t' || out[pos] == '\r' || out[pos] == '\n' || out[pos] == ';') {
		pos++
	}
	return pos
}

func isIdentStart(c byte) bool {
	return c == '_' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
}

func isIdentChar(c byte) bool {
	return isIdentStart(c) || (c >= '0' && c <= '9')
}
