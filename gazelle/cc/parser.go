// Parser extracts the narrow recognized source facts the C/C++ Gazelle
// extension needs for package-level ownership and strict dependency
// resolution.
//
// A C/C++ source or header contributes dependency references only through
// quoted includes (`#include "path/to/header.h"`). Angle includes
// (`#include <vector>`) are toolchain-provided and never produce an edge;
// the pinned rules_cc toolchain stays authoritative at execution time, so
// no standard-library manifest is needed. Comments, string literals, and
// character literals are inert: text that looks like an include or a `main`
// definition inside them never produces a fact.
//
// Recognition is by a narrow comment/string-stripping scanner plus a
// `#include` line matcher, never by a full C++ grammar: backslash-newline
// continuations are joined before matching so a split include still counts,
// and a `main` definition is recognized as the token `main` followed by an
// opening parenthesis outside comments and literals. Generation never
// type-checks a file.
//
// Identity normalization: every quoted include contributes its basename
// (`cc/tests/fixtures/hello/hello.h` -> `hello.h`), which matches the owning library's
// indexed header basename. Two libraries owning the same header basename
// are ambiguous and fail resolution; owners add an exact
// `# gazelle:resolve` mapping or rename.
package cc

import (
	"path"
	"regexp"
	"sort"
	"strings"
)

var (
	// includeRe matches one stripped `#include "quoted"` line and captures
	// the quoted path. Angle includes never match and produce no edge.
	includeRe = regexp.MustCompile(`(?m)^\s*#\s*include\s*"([^"]+)"`)
	// mainRe matches the token `main` followed by an opening parenthesis
	// outside comments and literals. It is deliberately narrow: any
	// non-test source defining `main` keeps the directory handwritten
	// (thin `cc_binary` entries are never inferred).
	mainRe = regexp.MustCompile(`(^|[^A-Za-z0-9_])main\s*\(`)
)

// stripNonCode returns content with comments, string literals, character
// literals, and raw strings replaced by spaces (newlines preserved so line
// structure survives). Backslash-newline continuations are joined first so
// a split directive still matches.
func stripNonCode(content []byte) []byte {
	s := strings.ReplaceAll(string(content), "\\\r\n", " ")
	s = strings.ReplaceAll(s, "\\\n", " ")
	out := make([]byte, len(s))
	copy(out, s)
	mask := func(from, to int) {
		for i := from; i < to; i++ {
			if out[i] != '\n' {
				out[i] = ' '
			}
		}
	}
	i := 0
	for i < len(out) {
		c := out[i]
		switch {
		case c == '/' && i+1 < len(out) && out[i+1] == '/':
			j := i + 2
			for j < len(out) && out[j] != '\n' {
				j++
			}
			mask(i, j)
			i = j
		case c == '/' && i+1 < len(out) && out[i+1] == '*':
			j := i + 2
			for j+1 < len(out) && !(out[j] == '*' && out[j+1] == '/') {
				j++
			}
			if j+1 < len(out) {
				j += 2
			} else {
				j = len(out)
			}
			mask(i, j)
			i = j
		case c == '"' || c == '\'':
			quote := c
			j := i + 1
			// Raw strings (R"delim(...)delim") start with R" outside
			// identifiers; treat the R prefix as part of the literal.
			if quote == '"' && i > 0 && out[i-1] == 'R' && (i < 2 || !isIdentChar(out[i-2])) {
				mask(i-1, i)
			}
			closed := false
			for j < len(out) {
				if out[j] == '\\' {
					j += 2
					continue
				}
				if out[j] == quote {
					// A single-quoted multi-character sequence is a
					// multi-character literal, still inert for facts.
					closed = true
					j++
					break
				}
				if quote == '"' && out[j] == '\n' {
					break
				}
				j++
			}
			if closed {
				mask(i, j)
				i = j
			} else {
				// Unterminated literal: mask to end of line and move on
				// rather than guessing.
				k := i
				for k < len(out) && out[k] != '\n' {
					k++
				}
				mask(i, k)
				i = k
			}
		default:
			i++
		}
	}
	return out
}

func isIdentChar(c byte) bool {
	return c == '_' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
}

// ParseQuotedIncludes returns the sorted unique header-basename identities
// for one C/C++ source or header file. Angle includes are ignored
// (toolchain-provided, never edges). Comment- or literal-embedded text that
// looks like an include never produces an edge.
func ParseQuotedIncludes(content []byte) []string {
	set := make(map[string]struct{})
	// Join backslash-newline continuations first so a split directive
	// still matches; newlines are preserved elsewhere by the scanner.
	s := strings.ReplaceAll(string(content), "\\\r\n", " ")
	s = strings.ReplaceAll(s, "\\\n", " ")
	b := []byte(s)
	n := len(b)
	i := 0
	for i < n {
		c := b[i]
		// Line comment.
		if c == '/' && i+1 < n && b[i+1] == '/' {
			j := i + 2
			for j < n && b[j] != '\n' {
				j++
			}
			i = j
			continue
		}
		// Block comment.
		if c == '/' && i+1 < n && b[i+1] == '*' {
			j := i + 2
			for j+1 < n && !(b[j] == '*' && b[j+1] == '/') {
				j++
			}
			if j+1 < n {
				i = j + 2
			} else {
				break
			}
			continue
		}
		// Raw string R"delim(...)delim" outside identifiers.
		if c == 'R' && i+1 < n && b[i+1] == '"' && (i == 0 || !isIdentChar(b[i-1])) {
			if end, ok := skipRawString(b, i); ok {
				i = end
				continue
			}
			// Fall through to ordinary string handling when not a raw string.
		}
		// Ordinary string or character literal: inert.
		if c == '"' || c == '\'' {
			i = skipQuoted(b, i)
			continue
		}
		// Preprocessor directive opener outside comments and literals.
		if c == '#' {
			if isDirectiveStart(b, i) && hasIncludeKeyword(b, i+1) {
				if raw, next, ok := parseIncludePath(b, i+1); ok {
					if raw != "" {
						set[path.Base(raw)] = struct{}{}
					}
					i = next
					continue
				}
			}
			i++
			continue
		}
		i++
	}
	out := make([]string, 0, len(set))
	for name := range set {
		out = append(out, name)
	}
	sort.Strings(out)
	return out
}

// isDirectiveStart reports whether the '#' at pos opens a preprocessor
// directive: only whitespace precedes it back to the previous newline.
func isDirectiveStart(b []byte, pos int) bool {
	j := pos - 1
	for j >= 0 && b[j] != '\n' {
		if b[j] != ' ' && b[j] != '\t' && b[j] != '\r' {
			return false
		}
		j--
	}
	return true
}

// hasIncludeKeyword reports whether the bytes after '#' spell `include`
// with word boundaries.
func hasIncludeKeyword(b []byte, pos int) bool {
	p := pos
	for p < len(b) && (b[p] == ' ' || b[p] == '\t' || b[p] == '\r') {
		p++
	}
	if p+7 > len(b) || string(b[p:p+7]) != "include" {
		return false
	}
	after := p + 7
	if after < len(b) && isIdentChar(b[after]) {
		return false
	}
	return true
}

// parseIncludePath parses after the '#' of a `#include` directive known to
// carry the keyword: it skips to the quoted or angle path and returns the
// raw quoted path (empty for angle includes, which are toolchain-provided).
// It returns the offset to resume scanning from and whether the directive
// was a well-formed include line.
func parseIncludePath(b []byte, hashPos int) (string, int, bool) {
	p := hashPos
	for p < len(b) && (b[p] == ' ' || b[p] == '\t' || b[p] == '\r') {
		p++
	}
	if p+7 > len(b) || string(b[p:p+7]) != "include" {
		return "", p, false
	}
	p += 7
	for p < len(b) && (b[p] == ' ' || b[p] == '\t' || b[p] == '\r') {
		p++
	}
	if p >= len(b) {
		return "", p, false
	}
	if b[p] == '"' {
		j := p + 1
		for j < len(b) && b[j] != '"' && b[j] != '\n' {
			if b[j] == '\\' && j+1 < len(b) {
				j += 2
				continue
			}
			j++
		}
		if j >= len(b) || b[j] != '"' {
			return "", j, false
		}
		raw := strings.TrimSpace(string(b[p+1 : j]))
		return raw, j + 1, true
	}
	if b[p] == '<' {
		j := p + 1
		for j < len(b) && b[j] != '>' && b[j] != '\n' {
			j++
		}
		if j < len(b) && b[j] == '>' {
			return "", j + 1, true
		}
		return "", j, false
	}
	return "", p, false
}

// skipQuoted returns the offset just past the string or character literal
// opening at pos, or the end of the line (for strings) or buffer when
// unterminated.
func skipQuoted(b []byte, pos int) int {
	quote := b[pos]
	j := pos + 1
	for j < len(b) {
		if b[j] == '\\' {
			j += 2
			continue
		}
		if b[j] == quote {
			return j + 1
		}
		if quote == '"' && b[j] == '\n' {
			return j
		}
		j++
	}
	return len(b)
}

// skipRawString returns the offset just past the raw string opening at pos
// (the `R` of `R"delim(... )delim"`), or not-ok when pos does not open a
// raw string.
func skipRawString(b []byte, pos int) (int, bool) {
	// pos points at 'R', pos+1 is '"'.
	q := pos + 2
	for q < len(b) && b[q] != '(' {
		if b[q] == ' ' || b[q] == '\t' || b[q] == '\r' || b[q] == '\n' || b[q] == '\\' {
			return 0, false
		}
		q++
	}
	if q >= len(b) || b[q] != '(' {
		return 0, false
	}
	delim := string(b[pos+2 : q])
	closer := ")" + delim + "\""
	rest := string(b[q+1:])
	idx := strings.Index(rest, closer)
	if idx < 0 {
		return len(b), true
	}
	return q + 1 + idx + len(closer), true
}

// DefinesMain reports whether a C/C++ source defines a `main` entry point
// outside comments and literals. Test-owned sources are never asked;
// callers fail generation for a `main`-defining library source rather than
// inferring a thin binary.
func DefinesMain(content []byte) bool {
	return mainRe.Match(stripNonCode(content))
}
