// Naming implements the deterministic TypeScript target-name normalizer owned
// by the first-party TypeScript Gazelle extension.
//
// The durable constraint comes from the common generation contract:
// basename-derived names preserve ASCII letters, ASCII digits, and internal
// underscores; every run of any other character becomes one underscore;
// leading and trailing underscores are trimmed; an empty result fails
// generation. Same-package normalized-name collisions fail with every
// claimant; the extension never invents a language affix or another suffix.
//
// A `.ts`/`.tsx`/`.mts`/`.cts` source is a test only when its basename ends
// in `_test` immediately before its language extension. A `test_` prefix,
// a `.test.` infix, test-directory placement, runner configuration, and
// test functions do not create automatic test ownership. Declaration files
// (`.d.ts`, `.d.mts`, `.d.cts`), source maps, and framework containers never
// create TypeScript ownership.
package typescript

import (
	"fmt"
	"path"
	"strings"
)

// SupportedExts are the core TypeScript source extensions discovered by the
// extension. Declaration files, source maps, and framework containers are
// inert and never listed here.
var SupportedExts = []string{".ts", ".tsx", ".mts", ".cts"}

// IsDeclaration reports whether a source path is an inert TypeScript
// declaration file (`.d.ts`, `.d.mts`, `.d.cts`). Declarations never create
// ownership, test identity, or entry identity.
func IsDeclaration(name string) bool {
	base := path.Base(name)
	return strings.HasSuffix(base, ".d.ts") || strings.HasSuffix(base, ".d.mts") || strings.HasSuffix(base, ".d.cts")
}

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

// stripTSExt reports the basename without its final TypeScript extension.
// It returns ok=false when the name carries no supported extension. Callers
// filter declarations first; a declaration stem (e.g. `foo.d`) is never a
// valid ownership input.
func stripTSExt(base string) (string, bool) {
	for _, ext := range SupportedExts {
		if strings.HasSuffix(base, ext) {
			return strings.TrimSuffix(base, ext), true
		}
	}
	return "", false
}

// IsTestFile reports whether a directory-relative source path is a
// recognized TypeScript test source: a supported non-declaration file whose
// basename ends in `_test` immediately before its language extension. A
// `test_` prefix, a `.test.` infix, directory placement, and runner
// configuration never create test ownership.
func IsTestFile(name string) bool {
	if IsDeclaration(name) {
		return false
	}
	stem, ok := stripTSExt(path.Base(name))
	if !ok {
		return false
	}
	return strings.HasSuffix(stem, "_test")
}

// TargetName derives the Bazel target name for one TypeScript source path:
// the basename without its final extension, normalized. Test sources retain
// their `_test` suffix; the extension never appends another affix.
func TargetName(name string) (string, error) {
	base := path.Base(name)
	stem, ok := stripTSExt(base)
	if !ok {
		return Normalize(base)
	}
	return Normalize(stem)
}

// ModuleName returns the import identity for one TypeScript source path:
// the basename without its final extension, exact and unnormalized.
// Relative references resolve by this stem; bare specifiers resolve through
// authoritative pnpm scope or exact mappings.
func ModuleName(name string) string {
	base := path.Base(name)
	if stem, ok := stripTSExt(base); ok {
		return stem
	}
	return base
}

// IsEntryFile reports whether a directory-relative source path is a
// recognized executable entry in the narrow M16 slice: exactly `main.ts`,
// `main.tsx`, `main.mts`, or `main.cts` (non-test, non-declaration).
// Manifest-declared names win only for exact mappings; other layouts remain
// out of scope. Thin-binary generation is deferred until the execution
// wrapper lands; entries currently own only their library.
func IsEntryFile(name string) bool {
	if IsTestFile(name) || IsDeclaration(name) {
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

// Claimant records one generated or handwritten target competing for a
// normalized name in a single Bazel package.
type Claimant struct {
	// Name is the normalized target name under contention.
	Name string
	// Source identifies the claimant for diagnostics: a source path for
	// generated targets, "handwritten:<label>" for existing BUILD rules.
	Source string
	// Kind is the generated rule kind claiming the name. Empty means
	// infer project ownership; set explicitly when callers distinguish
	// roles sharing one kind.
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
