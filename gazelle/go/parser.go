// Parser extracts the narrow recognized source facts the Go Gazelle
// extension needs for package-level ownership and strict dependency
// resolution.
//
// A `.go` source file carries one package clause and zero or more import
// specs. Only the import specs contribute dependency references: comments,
// string literals, and all other code never do, and the pinned rules_go
// toolchain stays authoritative at execution time.
//
// Recognition is by the standard `go/parser` with `ImportsOnly` (plus a
// `PackageClauseOnly` pass for the package identity), never by regular
// expression: comment-embedded or string-embedded text that looks like an
// import never produces an edge, and unparseable files fail generation
// loudly instead of contributing a guessed edge set. Generation never
// type-checks a file.
//
// Specifier normalization: third-party module imports (a dot in the first
// path segment, e.g. `github.com/x/y`) keep their full literal path and
// resolve only through an exact `# gazelle:resolve` mapping; standard
// library paths are included and filtered by callers via IsStdLib; every
// other import is local and contributes its final path segment
// (`rules_dx/go/tests/fixtures/hello` -> `hello`), which matches the owning library's
// indexed module stem. Two local packages sharing one final segment are
// ambiguous and fail resolution; owners add an exact mapping or rename.
package golang

import (
	"go/parser"
	"go/token"
	"path"
	"sort"
	"strconv"
	"strings"
)

// ParseImports returns the sorted unique normalized import roots for one Go
// source file. Standard-library identities are included; callers filter
// them via IsStdLib. Unparseable files fail with an error; callers fail
// generation rather than guessing.
func ParseImports(content []byte) ([]string, error) {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "source.go", content, parser.ImportsOnly)
	if err != nil {
		return nil, err
	}
	set := make(map[string]struct{})
	for _, imp := range f.Imports {
		raw, err := strconv.Unquote(imp.Path.Value)
		if err != nil || strings.TrimSpace(raw) == "" {
			continue
		}
		if root := normalizeImport(raw); root != "" {
			set[root] = struct{}{}
		}
	}
	out := make([]string, 0, len(set))
	for name := range set {
		out = append(out, name)
	}
	sort.Strings(out)
	return out, nil
}

// ParsePackage returns the package clause identity for one Go source file.
// Unparseable files fail with an error; callers fail generation rather
// than guessing.
func ParsePackage(content []byte) (string, error) {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, "source.go", content, parser.PackageClauseOnly)
	if err != nil {
		return "", err
	}
	return f.Name.Name, nil
}

// normalizeImport maps one literal import path to its resolution root:
// full literal for third-party module paths, final segment for local
// paths, unchanged for standard library (filtered by callers).
func normalizeImport(spec string) string {
	spec = strings.TrimSpace(spec)
	if spec == "" {
		return ""
	}
	first := spec
	if i := strings.IndexByte(spec, '/'); i >= 0 {
		first = spec[:i]
	}
	if strings.Contains(first, ".") {
		return spec
	}
	if IsStdLib(spec) {
		return spec
	}
	return path.Base(spec)
}
