// Parser extracts the narrow recognized source facts the Java Gazelle
// extension needs for package-level ownership and strict dependency
// resolution.
//
// A `.java` source contributes dependency references only through import
// declarations (`import com.example.Foo;`, `import static ...;`, and
// on-demand `import com.example.*;`). The package clause contributes the
// source's own package identity, never an edge. Comments, string literals,
// character literals, and text blocks are inert: text that looks like an
// import or a `main` definition inside them never produces a fact.
//
// Recognition is by a narrow comment/string-stripping scanner plus an
// import-statement matcher, never by a full Java grammar. Generation never
// type-checks a file.
//
// Identity normalization: every non-stdlib import contributes its simple
// class name (final dot segment; `com.example.Foo` -> `Foo`, on-demand
// `com.example.*` -> `example`), which matches the owning library's
// indexed simple names. JDK imports (`java.*`, `javax.*`, `javafx.*`,
// `jdk.*`, `org.w3c.*`, `org.xml.*`) are included and filtered by callers
// via IsStdLib. Two libraries owning the same simple name are ambiguous
// and fail resolution; owners add an exact `# gazelle:resolve` mapping or
// rename.
package java

import (
	"path"
	"regexp"
	"sort"
	"strings"
)

var (
	// importRe matches one stripped `import [static] path[. *] ;` statement
	// and captures the dotted path plus an optional `.*` marker. It runs
	// over comment/string-stripped content, so embedded text never matches.
	importRe = regexp.MustCompile(`(?m)^\s*import\s+(?:static\s+)?([A-Za-z_][\w]*(?:\.[\w]+)*)(\.\*)?\s*;`)
	// packageRe matches one stripped `package path;` clause and captures
	// the dotted path.
	packageRe = regexp.MustCompile(`(?m)^\s*package\s+([A-Za-z_][\w]*(?:\.[\w]+)*)\s*;`)
	// mainRe matches the `static void main(` token sequence outside
	// comments and literals. It is deliberately narrow: any non-test
	// source defining `main` keeps the directory handwritten (thin
	// `dx_java_binary` entries are never inferred).
	mainRe = regexp.MustCompile(`static\s+void\s+main\s*\(`)
)

// stripNonCode returns content with line comments, block comments, string
// literals, character literals, and text blocks replaced by spaces
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
		case c == '"' && i+2 < len(out) && out[i+1] == '"' && out[i+2] == '"':
			// Text block: mask through the closing delimiter.
			rest := strings.Index(string(out[i+3:]), `"""`)
			if rest < 0 {
				mask(i, len(out))
				i = len(out)
			} else {
				mask(i, i+3+rest+3)
				i = i + 3 + rest + 3
			}
		case c == '"' || c == '\'':
			quote := c
			j := i + 1
			closed := false
			for j < len(out) {
				if out[j] == '\\' {
					j += 2
					continue
				}
				if out[j] == quote {
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
// one Java source file. JDK identities are included; callers filter them
// via IsStdLib. Comment- or literal-embedded text that looks like an
// import never produces an edge.
func ParseImports(content []byte) []string {
	stripped := stripNonCode(content)
	set := make(map[string]struct{})
	for _, m := range importRe.FindAllSubmatch(stripped, -1) {
		dotted := string(m[1])
		onDemand := len(m[2]) > 0
		if id := normalizeImport(dotted, onDemand); id != "" {
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

// ParsePackage returns the package-clause identity for one Java source
// file, or "" when the file carries no package clause (default package).
// A file with two package clauses fails closed.
func ParsePackage(content []byte) (string, error) {
	stripped := stripNonCode(content)
	matches := packageRe.FindAllSubmatch(stripped, -1)
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
	return "java: duplicate package clause " + e.first
}

func errDuplicatePackage(first string) error { return &duplicatePackageError{first: first} }

// normalizeImport maps one dotted import path to its resolution identity:
// the final dot segment for single-type imports, the final package segment
// for on-demand imports. JDK paths are returned unchanged for caller-side
// filtering.
func normalizeImport(dotted string, onDemand bool) string {
	dotted = strings.TrimSpace(dotted)
	if dotted == "" {
		return ""
	}
	if IsStdLib(dotted) {
		return dotted
	}
	if onDemand {
		return path.Base(strings.ReplaceAll(dotted, ".", "/"))
	}
	if i := strings.LastIndexByte(dotted, '.'); i >= 0 {
		return dotted[i+1:]
	}
	return dotted
}

// DefinesMain reports whether a Java source defines a `main` entry point
// outside comments and literals. Test-owned sources are never asked;
// callers fail generation for a `main`-defining library source rather than
// inferring a thin binary.
func DefinesMain(content []byte) bool {
	return mainRe.Match(stripNonCode(content))
}
