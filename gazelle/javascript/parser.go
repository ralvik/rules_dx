// Parser extracts the narrow recognized source facts the JavaScript Gazelle
// extension needs for one-source ownership and strict dependency resolution.
//
// Recognized syntax (M16 narrow source-only scope; additional forms require
// parser fixtures before they become recognized):
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
// their basename without the final extension (`./hello.js` -> `hello`);
// bare specifiers contribute the full literal (`react`, `@scope/pkg`,
// `pkg/subpath`); `node:`-prefixed and builtin identities are included and
// filtered by callers via IsStdLib.
package javascript

import (
	"path"
	"sort"
	"strings"
)

// ParseImports returns the sorted unique normalized import roots for one
// JavaScript source file. Standard-library identities are included; callers
// filter them via IsStdLib.
func ParseImports(content []byte) []string {
	set := make(map[string]struct{})
	add := func(spec string) {
		root := normalizeSpec(spec)
		if root == "" {
			return
		}
		set[root] = struct{}{}
	}
	scan(content, add)
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

// scan walks the source in one pass, skipping comments and inert literals,
// and reports each recognized literal specifier via add.
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
		// expected, skip to the closing unescaped `/`. This only avoids
		// false `import` matches inside regexes; misses fall closed by
		// failing to parse, never by inventing an edge.
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

// parseImport parses at an `import` keyword (ident ends at pos). It handles
// side-effect imports, static `from` imports, `import.meta` (no edge), and
// literal dynamic `import("name")`. It returns the offset to resume from.
func parseImport(src []byte, kwStart, pos int, add func(string)) int {
	n := len(src)
	_ = kwStart
	p := skipTrivia(src, pos)
	if p < n && src[p] == '.' {
		// import.meta: no edge.
		return p + 1
	}
	if p < n && src[p] == '(' {
		// Dynamic import: literal string only.
		p++
		p = skipTrivia(src, p)
		if p < n && (src[p] == '\'' || src[p] == '"') {
			if spec, next, ok := parseQuoted(src, p); ok {
				q := skipTrivia(src, next)
				if q < n && src[q] == ')' {
					add(spec)
					return q + 1
				}
				return next
			}
			return p + 1
		}
		return p
	}
	// Optional `type` modifier (`import type ...` in TS-flavored JS).
	if hasWordAt(src, p, "type") {
		p = skipTrivia(src, p+4)
	}
	// Side-effect import: string literal directly.
	if p < n && (src[p] == '\'' || src[p] == '"') {
		if spec, next, ok := parseQuoted(src, p); ok {
			add(spec)
			return next
		}
		return p + 1
	}
	// Static form: scan tokens until `from` + string, `;`, or newline
	// boundary. Braces and `*`/`as`/identifiers are skipped; a string
	// before `from` is inert (it can only be a malformed tail).
	for p < n {
		if src[p] == '\'' || src[p] == '"' {
			// A string here is not preceded by `from`; skip it inertly.
			p = skipQuoted(src, p)
			continue
		}
		if src[p] == '`' {
			p = skipTemplate(src, p)
			continue
		}
		if src[p] == ';' {
			return p + 1
		}
		if src[p] == '\n' {
			// Static imports may span lines inside braces, but a bare
			// newline outside braces without `from` ends the statement.
			// Narrow: continue scanning; termination comes from `;`,
			// `from`-string, or end. Newlines alone never emit.
			p++
			continue
		}
		if isIdentStart(src[p]) {
			j := p + 1
			for j < n && isIdentChar(src[j]) {
				j++
			}
			if string(src[p:j]) == "from" {
				q := skipTrivia(src, j)
				if q < n && (src[q] == '\'' || src[q] == '"') {
					if spec, next, ok := parseQuoted(src, q); ok {
						add(spec)
						return next
					}
					return q + 1
				}
				return j
			}
			p = j
			continue
		}
		p++
	}
	return p
}

// parseExport parses after an `export` keyword. Only `... from "name"`
// re-exports produce an edge.
func parseExport(src []byte, pos int, add func(string)) int {
	n := len(src)
	p := skipTrivia(src, pos)
	if hasWordAt(src, p, "type") {
		p = skipTrivia(src, p+4)
	}
	// `export *`, `export * as ns`, `export { ... }`: scan to `from`.
	// Any other form (`export const`, `export default`, `export function`,
	// `export class`, `export =`) produces no edge.
	depth := 0
	for p < n {
		if src[p] == '\'' || src[p] == '"' {
			p = skipQuoted(src, p)
			continue
		}
		if src[p] == '`' {
			p = skipTemplate(src, p)
			continue
		}
		if src[p] == '{' {
			depth++
			p++
			continue
		}
		if src[p] == '}' {
			if depth > 0 {
				depth--
			}
			p++
			continue
		}
		if src[p] == ';' || src[p] == '\n' {
			// A semicolon before `from` ends a non-re-export. Newlines
			// inside braces never terminate; newlines outside braces end
			// the declaration unless the next token is `from` (covers
			// `} from "..."` split across lines).
			if src[p] == ';' {
				return p + 1
			}
			if depth > 0 {
				p++
				continue
			}
			q := skipTrivia(src, p)
			if hasWordAt(src, q, "from") {
				p = q
				continue
			}
			return q
		}
		if isIdentStart(src[p]) {
			j := p + 1
			for j < n && isIdentChar(src[j]) {
				j++
			}
			if string(src[p:j]) == "from" {
				q := skipTrivia(src, j)
				if q < n && (src[q] == '\'' || src[q] == '"') {
					if spec, next, ok := parseQuoted(src, q); ok {
						add(spec)
						return next
					}
					return q + 1
				}
				return j
			}
			p = j
			continue
		}
		p++
	}
	return p
}

// parseRequire parses after a `require` identifier. Only a single literal
// string argument produces an edge.
func parseRequire(src []byte, pos int, add func(string)) int {
	n := len(src)
	p := skipTrivia(src, pos)
	if p >= n || src[p] != '(' {
		return pos
	}
	p++
	p = skipTrivia(src, p)
	if p < n && (src[p] == '\'' || src[p] == '"') {
		if spec, next, ok := parseQuoted(src, p); ok {
			q := skipTrivia(src, next)
			if q < n && src[q] == ')' {
				add(spec)
				return q + 1
			}
			return next
		}
		return p + 1
	}
	return p
}

// skipTrivia advances past whitespace and comments.
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

// parseQuoted parses a single- or double-quoted literal at pos (which must
// be the opening quote). It reports the body and the offset just past the
// closing quote. Backslash escapes are honored. An unterminated literal
// (newline or EOF before the close) returns not-ok and consumes nothing
// beyond the opener.
func parseQuoted(src []byte, pos int) (string, int, bool) {
	n := len(src)
	quote := src[pos]
	k := pos + 1
	var b strings.Builder
	for k < n {
		c := src[k]
		if c == '\\' {
			if k+1 < n {
				// Preserve the escaped byte literally; specifiers never
				// need escape expansion for resolution roots.
				b.WriteByte(src[k+1])
				k += 2
				continue
			}
			return "", pos + 1, false
		}
		if c == quote {
			return b.String(), k + 1, true
		}
		if c == '\n' {
			return "", pos + 1, false
		}
		b.WriteByte(c)
		k++
	}
	return "", pos + 1, false
}

// skipQuoted skips a single- or double-quoted literal starting at pos.
func skipQuoted(src []byte, pos int) int {
	if _, next, ok := parseQuoted(src, pos); ok {
		return next
	}
	// Unterminated: run to the newline or end so trailing text stays inert.
	n := len(src)
	k := pos + 1
	for k < n && src[k] != '\n' {
		k++
	}
	return k
}

// skipTemplate skips a template literal starting at the opening backtick,
// honoring escapes and `${ ... }` interpolation nesting.
func skipTemplate(src []byte, pos int) int {
	n := len(src)
	k := pos + 1
	depth := 0
	for k < n {
		c := src[k]
		if c == '\\' {
			k += 2
			continue
		}
		if c == '`' && depth == 0 {
			return k + 1
		}
		if c == '$' && k+1 < n && src[k+1] == '{' {
			depth++
			k += 2
			continue
		}
		if c == '}' && depth > 0 {
			depth--
			k++
			continue
		}
		k++
	}
	return n
}

// isRegexStart heuristically reports whether a `/` at pos opens a regex
// literal rather than a comment or division. Narrow: true when the previous
// significant byte cannot end an expression (start, operators, punctuation,
// keywords like `return`). Misses only risk skipping an edge search inside
// the regex, never inventing one.
func isRegexStart(src []byte, pos int) bool {
	j := pos - 1
	for j >= 0 && (src[j] == ' ' || src[j] == '\t' || src[j] == '\r' || src[j] == '\n') {
		j--
	}
	if j < 0 {
		return true
	}
	c := src[j]
	switch c {
	case '(', ',', '=', ':', '[', '!', '&', '|', '?', '{', '}', ';', '+', '-', '*', '%', '<', '>', '^', '~':
		return true
	}
	// After `return`, `typeof`, etc. a regex may follow; approximate by
	// treating identifier-ends as division (no skip). This keeps the
	// heuristic conservative: regexes after keywords are scanned as code,
	// and `import`-looking text inside them could over-match. The parser
	// tests pin the accepted behavior; widen only with fixtures.
	return false
}

// skipRegex skips a regex literal starting at the opening `/`.
func skipRegex(src []byte, pos int) int {
	n := len(src)
	k := pos + 1
	inClass := false
	for k < n {
		c := src[k]
		if c == '\\' {
			k += 2
			continue
		}
		if c == '\n' {
			return k
		}
		if c == '[' {
			inClass = true
			k++
			continue
		}
		if c == ']' {
			inClass = false
			k++
			continue
		}
		if c == '/' && !inClass {
			k++
			for k < n && isIdentChar(src[k]) {
				k++
			}
			return k
		}
		k++
	}
	return n
}

// hasWordAt reports whether word starts at pos with a non-identifier
// boundary after it. The caller guarantees the leading boundary.
func hasWordAt(src []byte, pos int, word string) bool {
	if pos+len(word) > len(src) {
		return false
	}
	if string(src[pos:pos+len(word)]) != word {
		return false
	}
	after := pos + len(word)
	return after >= len(src) || !isIdentChar(src[after])
}

// isPrecededByDot reports whether the identifier at pos is immediately
// preceded by a dot (allowing no trivia): a method call, not a load.
func isPrecededByDot(src []byte, pos int) bool {
	j := pos - 1
	for j >= 0 && (src[j] == ' ' || src[j] == '\t' || src[j] == '\r' || src[j] == '\n') {
		j--
	}
	return j >= 0 && src[j] == '.'
}

func isIdentStart(c byte) bool {
	return c == '_' || c == '$' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
}

func isIdentChar(c byte) bool {
	return isIdentStart(c) || (c >= '0' && c <= '9')
}
