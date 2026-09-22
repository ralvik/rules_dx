// Parser extracts the narrow recognized source facts the Ruby Gazelle
// extension needs for package-level ownership and strict dependency
// resolution.
//
// A `.rb` source contributes dependency references only through `require`
// directives (`require "foo"`, `require 'foo/bar'`). `require_relative`
// references stay inside the one-package owner set and never become an
// edge. Comments are inert: text that looks like a `require` inside a
// comment never produces a fact. String-embedded text on non-require lines
// never matches because the matcher anchors at the line start.
//
// Recognition is by a narrow comment-stripping scanner plus a
// require-statement matcher, never by a full Ruby grammar. Generation never
// type-checks a file.
//
// Identity normalization: every non-stdlib `require` contributes its full
// require string (`foo/bar` stays `foo/bar`), which matches the owning
// library's indexed stems. Interpreter-bundled requires are included and
// filtered by callers via IsStdLib. Two libraries owning the same stem are
// ambiguous and fail resolution; owners add an exact
// `# gazelle:resolve` mapping or rename.
package ruby

import (
	"path"
	"regexp"
	"sort"
	"strings"
)

var (
	// requireRe matches one comment-stripped `require` line and captures
	// the quoted path. `require_relative`, `load`, `autoload`, and dynamic
	// requires are never matched here.
	requireRe = regexp.MustCompile(`(?m)^\s*require\s+["']([^"']+)["']\s*$`)
	// mainRe matches a Ruby executable guard outside comments.
	// Any non-test source defining it keeps the directory handwritten (thin
	// `ruby_binary` entries are never inferred).
	mainRe = regexp.MustCompile(`__FILE__\s*==\s*\$0|__FILE__\s*==\s*\$PROGRAM_NAME`)
)

// stripNonCode returns content with line comments (`#...`) and block
// comments (`=begin`/`=end`) replaced by spaces (newlines preserved so line
// structure survives). String literals stay intact so `require` paths
// remain matchable; the line-anchored matcher keeps string-embedded text
// on other lines inert.
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
	// Block comments first.
	lines := strings.Split(s, "\n")
	offset := 0
	offsets := make([]int, len(lines))
	for i, line := range lines {
		offsets[i] = offset
		offset += len(line) + 1
	}
	inBlock := false
	for i, line := range lines {
		trimmed := strings.TrimSpace(line)
		if !inBlock && (trimmed == "=begin" || strings.HasPrefix(trimmed, "=begin ")) {
			inBlock = true
			mask(offsets[i], offsets[i]+len(line))
			continue
		}
		if inBlock {
			mask(offsets[i], offsets[i]+len(line))
			if trimmed == "=end" || strings.HasPrefix(trimmed, "=end ") {
				inBlock = false
			}
			continue
		}
	}
	// Line comments (`#` outside strings is approximated by masking from
	// the first `#` that starts a comment; `#{}` interpolation is rare on
	// require lines and stays inert by anchoring).
	for i := 0; i < len(out); {
		if out[i] == '#' {
			j := i + 1
			for j < len(out) && out[j] != '\n' {
				j++
			}
			mask(i, j)
			i = j
			continue
		}
		i++
	}
	return out
}

// ParseImports returns the sorted unique require identities for one Ruby
// source file. Interpreter-bundled identities are included; callers filter
// them via IsStdLib. `require_relative` never produces an edge.
// Comment-embedded text that looks like a `require` never produces an edge.
func ParseImports(content []byte) []string {
	stripped := stripNonCode(content)
	set := make(map[string]struct{})
	for _, m := range requireRe.FindAllSubmatch(stripped, -1) {
		req := strings.TrimSpace(string(m[1]))
		if req == "" {
			continue
		}
		if strings.HasPrefix(req, ".") {
			continue
		}
		if id := normalizeImport(req); id != "" {
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

// ParsePackage returns the package identity for one Ruby source file.
// Ruby has no namespace declaration; every file belongs to its directory
// package, so this always returns "" and never fails.
func ParsePackage(content []byte) (string, error) {
	return "", nil
}

// normalizeImport maps one require string to its resolution identity:
// the full require path, unchanged. Stdlib paths are returned unchanged
// for caller-side filtering.
func normalizeImport(req string) string {
	return strings.TrimSpace(req)
}

// DefinesMain reports whether a Ruby source defines an executable guard
// (`if __FILE__ == $0` or `$PROGRAM_NAME`) outside comments. Test-owned
// sources are never asked; callers fail generation for a main-defining
// library source rather than inferring a thin binary.
func DefinesMain(content []byte) bool {
	return mainRe.Match(stripNonCode(content))
}

var _ = path.Base
