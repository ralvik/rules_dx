// Naming implements the deterministic JavaScript target-name normalizer owned
// by the first-party JavaScript Gazelle extension.
//
// The durable constraint comes from the common generation contract:
// basename-derived names preserve ASCII letters, ASCII digits, and internal
// underscores; every run of any other character becomes one underscore;
// leading and trailing underscores are trimmed; an empty result fails
// generation. Same-package normalized-name collisions fail with every
// claimant; the extension never invents a language affix or another suffix.
//
// A `.js`/`.jsx`/`.mjs`/`.cjs` source is a test only when its basename ends
// in `_test` immediately before its language extension. A `test_` prefix,
// a `.test.` infix, test-directory placement, runner configuration, and
// test functions do not create automatic test ownership. Declaration files
// (`.d.ts` and friends), source maps, and framework containers never create
// JavaScript ownership.
package javascript

import (
	"fmt"
	"path"
	"strings"
)

// SupportedExts are the core JavaScript source extensions discovered by the
// extension. Declaration files, source maps, and framework containers are
// inert and never listed here.
var SupportedExts = []string{".js", ".jsx", ".mjs", ".cjs"}

// Normalize maps one source basename (without its final language extension)
// to its deterministic Bazel target-name stem. It reports an error instead
// of an empty name so callers fail closed.
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

// stripJSExt reports the basename without its final JavaScript extension.
// It returns ok=false when the name carries no supported extension.
func stripJSExt(base string) (string, bool) {
	for _, ext := range SupportedExts {
		if strings.HasSuffix(base, ext) {
			return strings.TrimSuffix(base, ext), true
		}
	}
	return "", false
}

// IsTestFile reports whether a directory-relative source path is a
// recognized JavaScript test source: a supported file whose basename ends
// in `_test` immediately before its language extension. A `test_` prefix,
// a `.test.` infix, directory placement, and runner configuration never
// create test ownership.
func IsTestFile(name string) bool {
	stem, ok := stripJSExt(path.Base(name))
	if !ok {
		return false
	}
	return strings.HasSuffix(stem, "_test")
}

// TargetName derives the Bazel target name for one JavaScript source path:
// the basename without its final extension, normalized. Test sources retain
// their `_test` suffix; the extension never appends another affix.
func TargetName(name string) (string, error) {
	base := path.Base(name)
	stem, ok := stripJSExt(base)
	if !ok {
		return Normalize(base)
	}
	return Normalize(stem)
}

// ModuleName returns the import identity for one JavaScript source path:
// the basename without its final extension, exact and unnormalized.
// Relative references resolve by this stem; bare specifiers resolve through
// authoritative pnpm scope or exact mappings.
func ModuleName(name string) string {
	base := path.Base(name)
	if stem, ok := stripJSExt(base); ok {
		return stem
	}
	return base
}

// IsEntryFile reports whether a directory-relative source path is a
// recognized executable entry in the narrow M16 slice: exactly `main.js`,
// `main.jsx`, `main.mjs`, or `main.cjs` (non-test). Manifest-declared names
// win only for exact mappings; other layouts remain out of scope.
func IsEntryFile(name string) bool {
	if IsTestFile(name) {
		return false
	}
	base := path.Base(name)
	for _, ext := range SupportedExts {
		if base == "main"+ext {
			return true
		}
	}
	return false
}

// EntryBinaryName derives the thin-binary target name for one library
// owner: `<library>_bin`. Collisions with another library, test, or
// binary claim fail closed with every claimant; no further affix is
// invented.
func EntryBinaryName(lib string) string {
	return lib + "_bin"
}

// Claimant records one generated or handwritten target competing for a
// normalized name in a single Bazel package.
type Claimant struct {
	// Name is the normalized target name under contention.
	Name string
	// Source identifies the claimant for diagnostics: a source path for
	// generated targets, "handwritten:<label>" for existing BUILD rules.
	Source string
	// Kind is the generated rule kind claiming the name. Empty means
	// infer from the source (test when IsTestFile, else library); set it
	// explicitly for thin-binary claims whose source would infer library.
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
