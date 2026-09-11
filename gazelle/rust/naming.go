// Naming implements the deterministic Rust target-name normalizer owned
// by the first-party Rust Gazelle extension.
//
// The durable constraint comes from ADR 0004 and the common generation
// contract: basename-derived names preserve ASCII letters, ASCII digits,
// and internal underscores; every run of any other character (including
// non-ASCII characters) becomes one underscore; leading and trailing
// underscores are trimmed; an empty result fails generation. Same-package
// normalized-name collisions fail with every claimant; the extension never
// invents a language affix or another suffix.
package rust

import (
	"fmt"
	"strings"
)

// Normalize maps one source basename (without its final language
// extension) to its deterministic Bazel target-name stem. It reports an
// error instead of an empty name so callers fail closed.
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

// MustNormalize is Normalize for contexts where the input is already
// validated (crate-directory basenames from the filesystem walk). It
// panics on an empty result instead of threading an error through pure
// plumbing.
func MustNormalize(base string) string {
	out, err := Normalize(base)
	if err != nil {
		panic(err.Error())
	}
	return out
}

// Claimant records one generated or handwritten target competing for a
// normalized name in a single Bazel package.
type Claimant struct {
	// Name is the normalized target name under contention.
	Name string
	// Source identifies the claimant for diagnostics: a source path for
	// generated targets, "handwritten:<label>" for existing BUILD rules.
	Source string
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

// IntegrationTestName derives the `<stem>_test` target name for one
// direct `<crate-directory>/tests/*.rs` root. A stem that already ends in
// `_test` keeps its single suffix; an explicit authoritative Cargo
// `[[test]]` name wins before this function is consulted.
func IntegrationTestName(stem string) string {
	if strings.HasSuffix(stem, "_test") {
		return stem
	}
	return stem + "_test"
}

// UnitTestName derives the `<crate-target>_test` wrapper name for the one
// crate unit-test target of a library or executable crate.
func UnitTestName(crateTarget string) string {
	return crateTarget + "_test"
}

// BinaryName derives the `<crate-name>_bin` thin-binary name used when a
// source-only crate directory holds both `src/lib.rs` and `src/main.rs`
// and the library owns the fallback crate name.
func BinaryName(crateName string) string {
	return crateName + "_bin"
}

// ExampleName derives the `<name>_example` target name for an explicitly
// declared ordinary-binary Cargo `[[example]]` target.
func ExampleName(cargoName string) string {
	return MustNormalize(cargoName) + "_example"
}

// ExampleTestName derives the `<example-target>_test` crate-test wrapper
// name for an example declared with `test = true`.
func ExampleTestName(exampleTarget string) string {
	return exampleTarget + "_test"
}

// BenchName derives the `<name>_bench` target name for an explicitly
// declared ordinary-binary Cargo `[[bench]]` target with `harness = false`.
func BenchName(cargoName string) string {
	return MustNormalize(cargoName) + "_bench"
}

// BuildScriptName derives the `<package-name>_build_script` rule name for
// an active Cargo build script.
func BuildScriptName(packageName string) string {
	return MustNormalize(packageName) + "_build_script"
}
