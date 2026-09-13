// Naming implements the deterministic MDX target-name normalizer owned
// by the first-party MDX Gazelle extension.
//
// The durable constraint comes from the common generation contract:
// basename-derived names preserve ASCII letters, ASCII digits, and internal
// underscores; every run of any other character becomes one underscore;
// leading and trailing underscores are trimmed; an empty result fails
// generation. Same-package normalized-name collisions fail with every
// claimant; the extension never invents a language affix or another suffix.
//
// Every supported `.mdx` component receives one ordinary
// reusable one-source `dx_mdx_library`. MDX tests and entries stay
// JavaScript-owned (parsed or compiled outputs exercised through the
// `dx_js_test`/`dx_js_binary` wrappers); `.mdx` sources are never tests
// or thin binaries, so no test/entry inference exists here.
package mdx

import (
	"fmt"
	"path"
	"strings"
)

// SupportedExts are the MDX source extensions discovered by the
// extension. Core JS/TS sources and other framework containers are
// inert and never listed here.
var SupportedExts = []string{".mdx"}

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

// stripMDXExt reports the basename without its final MDX extension.
// It returns ok=false when the name carries no supported extension.
func stripMDXExt(base string) (string, bool) {
	for _, ext := range SupportedExts {
		if strings.HasSuffix(base, ext) {
			return strings.TrimSuffix(base, ext), true
		}
	}
	return "", false
}

// TargetName derives the Bazel target name for one MDX source path:
// the basename without its final extension, normalized.
func TargetName(name string) (string, error) {
	base := path.Base(name)
	stem, ok := stripMDXExt(base)
	if !ok {
		return Normalize(base)
	}
	return Normalize(stem)
}

// ModuleName returns the import identity for one MDX source path:
// the basename without its final extension, exact and unnormalized.
// Relative references resolve by this stem; bare specifiers resolve through
// authoritative pnpm scope or exact mappings.
func ModuleName(name string) string {
	base := path.Base(name)
	if stem, ok := stripMDXExt(base); ok {
		return stem
	}
	return base
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
