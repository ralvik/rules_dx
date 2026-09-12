// Parser extracts the narrow recognized source facts the Svelte Gazelle
// extension needs for one-source ownership and strict dependency resolution.
//
// A `.svelte` component carries template markup, script blocks, and a
// style block. Only `<script>` blocks contribute dependency references:
// the instance `<script>` and the module `<script context="module">`
// both execute as modules, so both resolve identically. Template markup
// and `<style>` regions are inert for dependency discovery.
//
// Recognition is by a narrow block parser, never by regular expression:
// ExtractScripts locates every `<script>` element with a tag scanner
// that understands HTML comments, quoted attribute values, and
// case-insensitive tag names, then returns each raw inner content.
// Generation never compiles the container.
//
// Recognized script syntax (narrow source-only scope; additional forms
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
// A component without a script block, or with an unparseable block
// boundary, contributes no imports and stays inert: it never falls back
// to a guessed JavaScript/TypeScript owner.
//
// Specifier normalization: relative (`./`, `../`, `/`) references contribute
// their basename without the final extension (`./helper.js` -> `helper`);
// bare specifiers contribute the full literal (`svelte`,
// `svelte/compiler`); `node:`-prefixed and builtin identities are included
// and filtered by callers via IsStdLib.
package svelte

import (
	"path"
	"sort"
	"strings"
)

// ParseImports returns the sorted unique normalized import roots for one
// Svelte component. Only `<script>` blocks are examined; template markup
// and style regions never contribute. Standard-library identities
// are included; callers filter them via IsStdLib.
func ParseImports(content []byte) []string {
	scripts := ExtractScripts(content)
	if len(scripts) == 0 {
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
	for _, script := range scripts {
		scan(script, add)
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

// ExtractScripts returns the raw inner content of every `<script>`
// element in a Svelte component, in source order, or nil when there is
// none or a block boundary is unparseable. Matching is case-insensitive
// for the tag name; the `script` name must end at a word boundary (space,
// `/`, or `>`). HTML comments (`<!--` ... `-->`) are skipped, and quoted
// attribute values may contain `>`. Self-closing `<script/>` elements
// carry no content and are skipped. An unparseable boundary makes the
// whole component inert (nil), never partially attributed.
func ExtractScripts(src []byte) [][]byte {
	n := len(src)
	var out [][]byte
	i := 0
	for i < n {
		// Skip HTML comments.
		if i+4 <= n && src[i] == '<' && src[i+1] == '!' && src[i+2] == '-' && src[i+3] == '-' {
			j := i + 4
			end := -1
			for j+2 < n {
				if src[j] == '-' && src[j+1] == '-' && src[j+2] == '>' {
					end = j + 3
					break
				}
				j++
			}
			if end < 0 {
				return nil
			}
			i = end
			continue
		}
		if src[i] != '<' {
			i++
			continue
		}
		// A closing tag at top level cannot open a script block.
		if i+1 < n && src[i+1] == '/' {
			i += 2
			continue
		}
		name, after, ok := scanTagName(src, i+1)
		if !ok {
			i++
			continue
		}
		if !equalFold(name, "script") {
			i = after
			continue
		}
		closePos, innerStart := scanTagEnd(src, after)
		if closePos < 0 {
			return nil
		}
		if src[closePos-1] == '/' {
			// Self-closing `<script/>` carries no content.
			i = innerStart
			continue
		}
		end := findCloseTag(src, innerStart, "script")
		if end < 0 {
			return nil
		}
		out = append(out, src[innerStart:end])
		i = end + len("</script>")
	}
	if len(out) == 0 {
		return nil
	}
	return out
}

// scanTagName reads a tag name starting at i (first name byte). It
// returns the name slice, the offset just past the name, and ok=false
// when no name is present.
func scanTagName(src []byte, i int) ([]byte, int, bool) {
	n := len(src)
	j := i
	for j < n && isTagNameChar(src[j]) {
		j++
	}
	if j == i {
		return nil, i, false
	}
	return src[i:j], j, true
}

// scanTagEnd scans from the offset past the tag name to the closing `>`
// of the open tag, honoring single- and double-quoted attribute values.
// It returns the offset of `>` and the offset just past it, or -1 when
// the tag never closes.
func scanTagEnd(src []byte, i int) (int, int) {
	n := len(src)
	j := i
	for j < n {
		c := src[j]
		if c == '"' || c == '\'' {
			k := j + 1
			for k < n && src[k] != c {
				k++
			}
			if k >= n {
				return -1, -1
			}
			j = k + 1
			continue
		}
		if c == '>' {
			return j, j + 1
		}
		j++
	}
	return -1, -1
}

// findCloseTag locates the opening `<` of the first well-formed closing
// tag `</name>` at or after start (case-insensitive), skipping HTML
// comments. It returns -1 when there is none.
func findCloseTag(src []byte, start int, name string) int {
	n := len(src)
	i := start
	for i < n {
		if i+4 <= n && src[i] == '<' && src[i+1] == '!' && src[i+2] == '-' && src[i+3] == '-' {
			j := i + 4
			end := -1
			for j+2 < n {
				if src[j] == '-' && src[j+1] == '-' && src[j+2] == '>' {
					end = j + 3
					break
				}
				j++
			}
			if end < 0 {
				return -1
			}
			i = end
			continue
		}
		if src[i] == '<' && i+1 < n && src[i+1] == '/' {
			raw, _, ok := scanTagName(src, i+2)
			if ok && equalFold(raw, name) {
				return i
			}
		}
		i++
	}
	return -1
}

func isTagNameChar(c byte) bool {
	return c == '_' || c == ':' || c == '-' ||
		(c >= 'a' && c <= 'z') ||
		(c >= 'A' && c <= 'Z') ||
		(c >= '0' && c <= '9')
}

func equalFold(a []byte, b string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := 0; i < len(a); i++ {
		ca := a[i]
		if ca >= 'A' && ca <= 'Z' {
			ca += 'a' - 'A'
		}
		if ca != b[i] {
			return false
		}
	}
	return true
}

// scan walks script content in one pass, skipping comments and inert
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
