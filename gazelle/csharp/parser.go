// Parser extracts the narrow recognized source facts the C# Gazelle
// extension needs for package-level ownership and strict dependency
// resolution.
//
// A `.cs` source contributes dependency references only through `using`
// directives (`using Foo.Bar;`, `using static Foo.Bar;`,
// `using Alias = Foo.Bar;`). The `namespace` declaration contributes the
// source's own namespace identity, never an edge. Comments, string literals
// (including verbatim and interpolated strings), and character literals are
// inert: text that looks like a `using` or `Main` inside them never produces
// a fact.
//
// Recognition is by a narrow comment/string-stripping scanner plus a
// using-statement matcher, never by a full C# grammar. Generation never
// type-checks a file.
//
// Identity normalization: every non-stdlib `using` contributes its simple
// name (final dot segment; `Foo.Bar.Baz` -> `Baz`), which matches the owning
// library's indexed simple names. `System.*`/`Microsoft.*` imports are
// included and filtered by callers via IsStdLib. Two libraries owning the
// same simple name are ambiguous and fail resolution; owners add an exact
// `# gazelle:resolve` mapping or rename.
package csharp

import (
	"path"
	"regexp"
	"sort"
	"strings"
)

var (
	// usingRe matches one stripped `using` directive and captures the dotted
	// path. `using static` and `using Alias =` prefixes are accepted; the
	// trailing semicolon is required by the language and required here.
	usingRe = regexp.MustCompile(`(?m)^\s*using\s+(?:static\s+)?(?:[A-Za-z_][\w]*\s*=\s*)?([A-Za-z_][\w]*(?:\.[\w]+)*)\s*;\s*$`)
	// namespaceRe matches one stripped `namespace` declaration (block or
	// file-scoped) and captures the dotted path.
	namespaceRe = regexp.MustCompile(`(?m)^\s*namespace\s+([A-Za-z_][\w]*(?:\.[\w]+)*)\b`)
	// mainRe matches `static ... Main(` outside comments and literals. Any
	// non-test source defining `Main` keeps the directory handwritten (thin
	// `dx_csharp_binary` entries are never inferred).
	mainRe = regexp.MustCompile(`static\s+[\w<>\[\],\s]*\bMain\s*\(`)
)

// stripNonCode returns content with line comments, block comments, string
// literals, verbatim strings, and character literals replaced by spaces
// (newlines preserved so line structure survives).
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
// one C# source file. System/Microsoft identities are included; callers
// filter them via IsStdLib. Comment- or literal-embedded text that looks
// like a `using` never produces an edge.
func ParseImports(content []byte) []string {
	stripped := stripNonCode(content)
	set := make(map[string]struct{})
	for _, m := range usingRe.FindAllSubmatch(stripped, -1) {
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

// ParsePackage returns the namespace identity for one C# source file, or ""
// when the file carries no namespace declaration (global namespace).
// A file with two namespace declarations fails closed.
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
	return "csharp: duplicate namespace declaration " + e.first
}

func errDuplicatePackage(first string) error { return &duplicatePackageError{first: first} }

// normalizeImport maps one dotted `using` path to its resolution identity:
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

// DefinesMain reports whether a C# source defines a `Main` entry point
// outside comments and literals. Test-owned sources are never asked;
// callers fail generation for a `Main`-defining library source rather than
// inferring a thin binary.
func DefinesMain(content []byte) bool {
	return mainRe.Match(stripNonCode(content))
}

var _ = path.Base
