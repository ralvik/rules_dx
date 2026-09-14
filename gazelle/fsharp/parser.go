// Parser extracts the narrow recognized source facts the F# Gazelle
// extension needs for package-level ownership and strict dependency
// resolution.
//
// An `.fs`/`.fsi` source contributes dependency references only through
// `open` directives (`open Foo.Bar`). The `namespace`/`module` declaration
// contributes the source's own namespace identity, never an edge. Comments,
// string literals, and character literals are inert: text that looks like an
// `open` or entry point inside them never produces a fact.
//
// Recognition is by a narrow comment/string-stripping scanner plus an
// open-statement matcher, never by a full F# grammar. Generation never
// type-checks a file.
//
// Identity normalization: every non-stdlib `open` contributes its simple
// name (final dot segment; `Foo.Bar.Baz` -> `Baz`), which matches the owning
// library's indexed simple names. `System.*`/`Microsoft.*` opens are
// included and filtered by callers via IsStdLib. Two libraries owning the
// same simple name are ambiguous and fail resolution; owners add an exact
// `# gazelle:resolve` mapping or rename.
package fsharp

import (
	"path"
	"regexp"
	"sort"
	"strings"
)

var (
	// openRe matches one stripped `open` directive and captures the dotted
	// path.
	openRe = regexp.MustCompile(`(?m)^\s*open\s+([A-Za-z_][\w]*(?:\.[\w]+)*)\s*$`)
	// namespaceRe matches one stripped `namespace` or `module` declaration
	// and captures the dotted path.
	namespaceRe = regexp.MustCompile(`(?m)^\s*(?:namespace|module)\s+([A-Za-z_][\w]*(?:\.[\w]+)*)\s*(?:=\s*)?$`)
	// mainRe matches the `[<EntryPoint>]` attribute outside comments and
	// literals. Any non-test source carrying it keeps the directory
	// handwritten (thin `fsharp_binary` entries are never inferred).
	mainRe = regexp.MustCompile(`\[<\s*EntryPoint\s*>\]`)
)

// stripNonCode returns content with line comments, block comments, string
// literals, and character literals replaced by spaces (newlines preserved
// so line structure survives).
func stripNonCode(content []byte) []byte {
	s := string(content)
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
		case c == '"':
			j := i + 1
			closed := false
			for j < len(out) {
				if out[j] == '\\' {
					j += 2
					continue
				}
				if out[j] == '"' {
					closed = true
					j++
					break
				}
				if out[j] == '\n' {
					break
				}
				j++
			}
			if closed {
				mask(i, j)
				i = j
			} else {
				k := i
				for k < len(out) && out[k] != '\n' {
					k++
				}
				mask(i, k)
				i = k
			}
		case c == '\'':
			j := i + 1
			closed := false
			for j < len(out) {
				if out[j] == '\\' {
					j += 2
					continue
				}
				if out[j] == '\'' {
					closed = true
					j++
					break
				}
				j++
			}
			if closed {
				mask(i, j)
				i = j
			} else {
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

// ParseImports returns the sorted unique normalized import identities for
// one F# source file. System/Microsoft identities are included; callers
// filter them via IsStdLib. Comment- or literal-embedded text that looks
// like an `open` never produces an edge.
func ParseImports(content []byte) []string {
	stripped := stripNonCode(content)
	set := make(map[string]struct{})
	for _, m := range openRe.FindAllSubmatch(stripped, -1) {
		dotted := string(m[1])
		if id := normalizeImport(dotted); id != "" {
			set[id] = struct{}{}
		}
	}
	out := make([]string, 0, len(set))
	for name := range set {
		out = append(out, name)
	}
	sort.Strings(out)
	return out
}

// ParsePackage returns the namespace/module identity for one F# source
// file, or "" when the file carries no declaration (global namespace).
// A file with two declarations fails closed.
func ParsePackage(content []byte) (string, error) {
	stripped := stripNonCode(content)
	matches := namespaceRe.FindAllSubmatch(stripped, -1)
	if len(matches) > 1 {
		return "", errDuplicatePackage(string(matches[0][1]))
	}
	if len(matches) == 0 {
		return "", nil
	}
	return string(matches[0][1]), nil
}

type duplicatePackageError struct{ first string }

func (e *duplicatePackageError) Error() string {
	return "fsharp: duplicate namespace/module declaration " + e.first
}

func errDuplicatePackage(first string) error { return &duplicatePackageError{first: first} }

// normalizeImport maps one dotted `open` path to its resolution identity:
// the final dot segment. System/Microsoft paths are returned unchanged for
// caller-side filtering.
func normalizeImport(dotted string) string {
	dotted = strings.TrimSpace(dotted)
	if dotted == "" {
		return ""
	}
	if IsStdLib(dotted) {
		return dotted
	}
	if i := strings.LastIndexByte(dotted, '.'); i >= 0 {
		return dotted[i+1:]
	}
	return dotted
}

// DefinesMain reports whether an F# source carries the `[<EntryPoint>]`
// attribute outside comments and literals. Test-owned sources are never
// asked; callers fail generation for an entry-point library source rather
// than inferring a thin binary.
func DefinesMain(content []byte) bool {
	return mainRe.Match(stripNonCode(content))
}

var _ = path.Base
