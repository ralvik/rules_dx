// Naming implements the deterministic Python target-name normalizer owned
// by the first-party Python Gazelle extension.
//
// The durable constraint comes from the common generation contract:
// basename-derived names preserve ASCII letters, ASCII digits, and internal
// underscores; every run of any other character becomes one underscore;
// leading and trailing underscores are trimmed; an empty result fails
// generation. Same-package normalized-name collisions fail with every
// claimant; the extension never invents a language affix or another suffix.
//
// A `.py` source is a test only when its basename ends in `_test`
// immediately before `.py`. A `test_` prefix, test-directory placement,
// pytest configuration, and test functions do not create automatic test
// ownership. A paired `.pyi` never creates a target.
package python

import (
	"fmt"
	"path"
	"strings"
)

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

// IsTestFile reports whether a directory-relative source path is a
// recognized Python test source: a `.py` file whose basename ends in
// `_test` immediately before `.py`. A `.pyi` stub, a `test_` prefix, and
// directory placement never create test ownership.
func IsTestFile(name string) bool {
	if !strings.HasSuffix(name, ".py") {
		return false
	}
	stem := strings.TrimSuffix(path.Base(name), ".py")
	return strings.HasSuffix(stem, "_test")
}

// TargetName derives the Bazel target name for one Python source path: the
// basename without its final extension, normalized. Test sources retain
// their `_test` suffix; the extension never appends another affix.
func TargetName(name string) (string, error) {
	base := path.Base(name)
	if strings.HasSuffix(base, ".py") {
		base = strings.TrimSuffix(base, ".py")
	} else if strings.HasSuffix(base, ".pyi") {
		base = strings.TrimSuffix(base, ".pyi")
	}
	return Normalize(base)
}

// ModuleName returns the top-level import identity for one Python source
// path: the basename without its final extension, exact and unnormalized.
// Dotted imports resolve by their root component, so only the stem matters.
func ModuleName(name string) string {
	base := path.Base(name)
	if strings.HasSuffix(base, ".py") {
		return strings.TrimSuffix(base, ".py")
	}
	if strings.HasSuffix(base, ".pyi") {
		return strings.TrimSuffix(base, ".pyi")
	}
	return base
}

// IsEntryFile reports whether a directory-relative source path is a
// recognized executable entry in the narrow M14 slice: exactly
// `main.py` (non-test). `__main__.py`, `if __name__ == "__main__"`
// guards, and manifest console scripts remain O25 qualification, not
// automatic recognition.
func IsEntryFile(name string) bool {
	if IsTestFile(name) {
		return false
	}
	return path.Base(name) == "main.py"
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
