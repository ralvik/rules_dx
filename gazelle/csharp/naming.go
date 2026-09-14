// Naming implements the deterministic CSharp target-name normalizer owned by
// the first-party CSharp Gazelle extension.
//
// The durable constraint comes from the common generation contract:
// basename-derived names preserve ASCII letters, ASCII digits, and internal
// underscores; every run of any other character becomes one underscore;
// leading and trailing underscores are trimmed; an empty result fails
// generation. Same-package normalized-name collisions fail with every
// claimant; the extension never invents a language affix or another suffix.
//
// CSharp is package-level (Go-style, not Python one-source): one directory
// holds one reusable `csharp_library` named after the directory basename,
// with `srcs` as the sorted non-test `.cs` files. `*Test.cs` files are
// never library sources (handwritten `csharp_test` owns them), thin
// `csharp_binary` entries are never inferred, and directories mixing a
// library with a `main`-defining source stay handwritten: generation fails
// and the owner must split the directory before adopting generated rules.
package csharp

import (
	"fmt"
	"path"
	"strings"
)

// SupportedExts are the CSharp source extensions discovered by the extension.
// Only `.cs` is listed; `*Test.cs` files are discovered then excluded
// from library sources (test-owned, never library-owned).
var SupportedExts = []string{".cs"}

// LibraryKind is the single generated rule kind.
const LibraryKind = "csharp_library"

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

// IsTestSource reports whether a `.cs` basename is test-owned: the stem
// (basename without its final extension) ends in `Test`. A `Test` prefix,
// test-directory placement, and `@Test` annotations do not create automatic
// test ownership by themselves.
func IsTestSource(name string) bool {
	base := path.Base(name)
	stem := strings.TrimSuffix(base, ".cs")
	return strings.HasSuffix(stem, "Test")
}

// DirTargetName derives the package-level library name for one directory:
// the directory basename, normalized.
func DirTargetName(dir string) (string, error) {
	return Normalize(path.Base(dir))
}

// TargetName derives the Bazel target name for one CSharp source path: the
// basename without its final extension, normalized. It is used for
// collision diagnostics and single-file fallback naming.
func TargetName(name string) (string, error) {
	base := path.Base(name)
	stem := strings.TrimSuffix(base, ".cs")
	return Normalize(stem)
}

// ClassIdentity returns the import identity for one CSharp import or owned
// source: the simple class name (final dot segment), exact and
// unnormalized. Two libraries owning the same simple name are ambiguous
// and fail resolution; owners add an exact mapping or rename.
func ClassIdentity(name string) string {
	base := path.Base(name)
	stem := strings.TrimSuffix(base, ".cs")
	if i := strings.LastIndex(stem, "."); i >= 0 {
		return stem[i+1:]
	}
	return stem
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
