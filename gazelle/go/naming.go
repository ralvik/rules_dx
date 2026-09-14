// Naming implements the deterministic Go target-name normalizer owned by
// the first-party Go Gazelle extension.
//
// The durable constraint comes from the common generation contract:
// basename-derived names preserve ASCII letters, ASCII digits, and internal
// underscores; every run of any other character becomes one underscore;
// leading and trailing underscores are trimmed; an empty result fails
// generation. Same-package normalized-name collisions fail with every
// claimant; the extension never invents a language affix or another suffix.
//
// Go is package-level (support-matrix Go exception): one directory holds one
// Go package (plus its external `*_test` package). The adapter generates at
// most one reusable `go_library` per directory named after the directory
// basename, with `srcs` as the sorted non-test `.go` files. `*_test.go`
// files are never library sources (handwritten `go_test` owns them via
// `embed`), thin `go_binary` entries are never inferred, and directories
// mixing a library package with a `package main` file stay handwritten:
// generation includes every non-test `.go` and the owner must split the
// directory before adopting generated rules.
package golang

import (
	"fmt"
	"path"
	"strings"
)

// SupportedExts are the Go source extensions discovered by the extension.
// Only `.go` is listed; `*_test.go` files are discovered then excluded from
// library sources (test-owned, never library-owned).
var SupportedExts = []string{".go"}

// LibraryKind is the single generated rule kind.
const LibraryKind = "go_library"

// Normalize maps one name stem to its deterministic Bazel target-name stem.
// It reports an error instead of an empty name so callers fail closed.
func Normalize(base string) (string, error) {
	var b strings.Builder
	b.Grow(len(base))
	pending := false
	for i := 0; i < len(base); i++ {
		c := base[i]
		if c <= 0x7F && (c == '_' ||
			(c >= 'a' && c <= 'z') ||
			(c >= 'A' && c <= 'Z') ||
			(c >= '0' && c <= '9')) {
			if pending && b.Len() > 0 {
				b.WriteByte('_')
			}
			pending = false
			b.WriteByte(c)
			continue
		}
		pending = true
	}
	out := strings.Trim(b.String(), "_")
	if out == "" {
		return "", fmt.Errorf("naming: %q normalizes to an empty target name", base)
	}
	return out, nil
}

// IsTestSource reports whether a `.go` basename is test-owned
// (`*_test.go`, including the external test package form).
func IsTestSource(name string) bool {
	base := path.Base(name)
	return strings.HasSuffix(base, "_test.go")
}

// DirTargetName derives the package-level library name for one directory:
// the directory basename, normalized.
func DirTargetName(dir string) (string, error) {
	return Normalize(path.Base(dir))
}

// TargetName derives the Bazel target name for one Go source path: the
// basename without its final extension, normalized. It is used for
// collision diagnostics and single-file fallback naming.
func TargetName(name string) (string, error) {
	base := path.Base(name)
	stem := strings.TrimSuffix(base, ".go")
	return Normalize(stem)
}

// ModuleName returns the import identity for one Go source path: the
// basename without its final extension, exact and unnormalized.
func ModuleName(name string) string {
	base := path.Base(name)
	return strings.TrimSuffix(base, ".go")
}

// Claimant records one generated or handwritten target competing for a
// normalized name in a single Bazel package.
type Claimant struct {
	// Name is the normalized target name under contention.
	Name string
	// Source identifies the claimant for diagnostics: a source path for
	// generated targets, "handwritten:<label>" for existing BUILD rules.
	Source string
	// Kind is the generated rule kind claiming the name.
	Kind string
}

// CollisionError reports a same-package normalized-name collision with
// every claimant. Generation fails rather than overwriting, dropping a
// target, or inventing a suffix.
type CollisionError struct {
	Name      string
	Claimants []string
}

func (e *CollisionError) Error() string {
	return fmt.Sprintf("naming: normalized name %q claimed by %s; rename a source or keep one target handwritten",
		e.Name, strings.Join(e.Claimants, ", "))
}

// CheckCollisions fails closed when two or more claimants share one
// normalized name. Claimants are grouped by Name; groups of one pass.
func CheckCollisions(claimants []Claimant) error {
	byName := make(map[string][]string, len(claimants))
	order := make([]string, 0, len(claimants))
	for _, c := range claimants {
		if _, ok := byName[c.Name]; !ok {
			order = append(order, c.Name)
		}
		byName[c.Name] = append(byName[c.Name], c.Source)
	}
	for _, name := range order {
		if sources := byName[name]; len(sources) > 1 {
			return &CollisionError{Name: name, Claimants: append([]string(nil), sources...)}
		}
	}
	return nil
}
