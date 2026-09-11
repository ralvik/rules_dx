// Layout discovers conventional source-only Rust crate shapes and loads
// crate module trees with single-owner semantics.
//
// A source-only crate uses recognized roots only: `<dir>/src/lib.rs`,
// `<dir>/src/main.rs`, and direct `<dir>/tests/*.rs` integration-test
// roots. Ownership is per crate, not per file: an authoritative root owns
// its root and every recursively loaded module, including explicit
// `#[path]` modules. A module is never rewritten as an independent crate.
// Ownership that cannot preserve Rust module semantics (orphan modules,
// `foo.rs` versus `foo/mod.rs` ambiguity, duplicate inclusion) fails
// generation instead of guessing.
package rust

import (
	"fmt"
	"path"
	"sort"
	"strings"
)

// CrateRoots are the recognized source-only roots of one crate directory.
// Paths are workspace-relative with `/` separators.
type CrateRoots struct {
	// Dir is the crate directory (`""` for the workspace root).
	Dir string
	// LibRoot is `<dir>/src/lib.rs` when present, else "".
	LibRoot string
	// BinRoot is `<dir>/src/main.rs` when present, else "".
	BinRoot string
	// TestRoots are direct `<dir>/tests/*.rs` files, bytewise sorted.
	TestRoots []string
}

// DiscoverCrateRoots finds the recognized source-only roots under dir.
// files lists workspace-relative candidate paths; only exact conventional
// roots match. No other standalone root, custom path, `src/bin` tree,
// example, benchmark, or build script is inferred without Cargo metadata.
func DiscoverCrateRoots(dir string, files []string) CrateRoots {
	roots := CrateRoots{Dir: dir}
	join := func(elem ...string) string {
		rel := path.Join(elem...)
		if dir == "" || dir == "." {
			return rel
		}
		return dir + "/" + rel
	}
	lib, bin := join("src", "lib.rs"), join("src", "main.rs")
	testPrefix := join("tests") + "/"
	for _, f := range files {
		switch {
		case f == lib:
			roots.LibRoot = f
		case f == bin:
			roots.BinRoot = f
		case strings.HasPrefix(f, testPrefix):
			rest := strings.TrimPrefix(f, testPrefix)
			if rest != "" && !strings.Contains(rest, "/") && strings.HasSuffix(rest, ".rs") {
				roots.TestRoots = append(roots.TestRoots, f)
			}
		}
	}
	sort.Strings(roots.TestRoots)
	return roots
}

// HasRoots reports whether the directory holds any recognized root.
func (r CrateRoots) HasRoots() bool {
	return r.LibRoot != "" || r.BinRoot != "" || len(r.TestRoots) > 0
}

// TestTarget is one generated integration-test target.
type TestTarget struct {
	// Name is the `<stem>_test` target name.
	Name string
	// Root is the workspace-relative integration-test root.
	Root string
}

// CrateShape is the generated target shape for one source-only crate
// directory: at most one library, at most one thin binary, and one target
// per direct integration-test root.
type CrateShape struct {
	// Dir is the crate directory.
	Dir string
	// Name is the fallback crate name (normalized directory basename).
	Name string
	// LibTarget is the library target name, empty without `src/lib.rs`.
	LibTarget string
	// BinTarget is the binary target name, empty without `src/main.rs`.
	BinTarget string
	// Tests holds one entry per direct integration-test root.
	Tests []TestTarget
	// LibUnitTest and BinUnitTest mark the one crate unit-test wrapper
	// (`<crate-target>_test`) for the library and binary respectively.
	LibUnitTest bool
	BinUnitTest bool
}

// ShapeCrate maps discovered roots to generated target names. libUnitTest
// and binUnitTest report whether the library and binary module trees
// contain recognized `#[test]` or `#[cfg(test)]` syntax; each true value
// yields exactly one crate unit-test wrapper. Same-shape normalized-name
// collisions fail with every claimant and no invented affix.
func ShapeCrate(roots CrateRoots, libUnitTest, binUnitTest bool) (*CrateShape, error) {
	if !roots.HasRoots() {
		return &CrateShape{Dir: roots.Dir}, nil
	}
	base := roots.Dir
	if base == "" || base == "." {
		return nil, fmt.Errorf("rust: crate directory %q has no source-only fallback name; an authoritative Cargo package name is required", roots.Dir)
	}
	name, err := Normalize(path.Base(base))
	if err != nil {
		return nil, fmt.Errorf("rust: crate directory %q: %w", roots.Dir, err)
	}
	shape := &CrateShape{Dir: roots.Dir, Name: name}
	var claimants []Claimant
	claim := func(target, source string) {
		claimants = append(claimants, Claimant{Name: target, Source: source})
	}
	if roots.LibRoot != "" {
		shape.LibTarget = name
		claim(name, roots.LibRoot)
	}
	if roots.BinRoot != "" {
		if roots.LibRoot != "" {
			shape.BinTarget = BinaryName(name)
		} else {
			shape.BinTarget = name
		}
		claim(shape.BinTarget, roots.BinRoot)
	}
	for _, root := range roots.TestRoots {
		stem, err := Normalize(strings.TrimSuffix(path.Base(root), ".rs"))
		if err != nil {
			return nil, fmt.Errorf("rust: integration-test root %s: %w", root, err)
		}
		target := IntegrationTestName(stem)
		shape.Tests = append(shape.Tests, TestTarget{Name: target, Root: root})
		claim(target, root)
	}
	if libUnitTest && roots.LibRoot != "" {
		target := UnitTestName(shape.LibTarget)
		shape.LibUnitTest = true
		claim(target, roots.LibRoot+" [unit]")
	}
	if binUnitTest && roots.BinRoot != "" {
		target := UnitTestName(shape.BinTarget)
		shape.BinUnitTest = true
		claim(target, roots.BinRoot+" [unit]")
	}
	if err := CheckCollisions(claimants); err != nil {
		return nil, err
	}
	return shape, nil
}

// LoadCrate loads the complete module tree of one crate or
// integration-test root. It returns every owned file (the root plus each
// recursively loaded module) mapped to its parsed facts. read supplies
// file bytes; exists reports checked-in presence for candidate module
// paths. Both candidates existing, no candidate existing, duplicate
// inclusion, and inclusion cycles fail instead of guessing an owner.
func LoadCrate(root string, read func(string) ([]byte, error), exists func(string) bool) (map[string]*FileFacts, error) {
	owned := make(map[string]*FileFacts)
	var visit func(file, moduleDir string, inheritedTest bool, stack []string) error
	visit = func(file, moduleDir string, inheritedTest bool, stack []string) error {
		for _, s := range stack {
			if s == file {
				return fmt.Errorf("rust: %s: module cycle through %s", root, strings.Join(append(stack, file), " -> "))
			}
		}
		if _, dup := owned[file]; dup {
			return fmt.Errorf("rust: %s: duplicate module inclusion of %s", root, file)
		}
		content, err := read(file)
		if err != nil {
			return fmt.Errorf("rust: %s: cannot read owned module %s: %v", root, file, err)
		}
		facts, err := ParseFacts(file, content)
		if err != nil {
			return err
		}
		if inheritedTest || facts.InnerCfgTest {
			facts.HasCfgTest = true
			for i := range facts.Modules {
				facts.Modules[i].CfgTest = true
			}
			for i := range facts.Uses {
				facts.Uses[i].CfgTest = true
			}
			for i := range facts.Externs {
				facts.Externs[i].CfgTest = true
			}
		}
		owned[file] = facts
		for _, m := range facts.Modules {
			if m.Inline {
				continue
			}
			var candidates []string
			if m.Path != "" {
				candidates = []string{joinRel(path.Dir(file), m.Path)}
			} else {
				candidates = []string{
					joinRel(moduleDir, m.Name+".rs"),
					joinRel(moduleDir, m.Name+"/mod.rs"),
				}
			}
			var found []string
			for _, c := range candidates {
				if exists(c) {
					found = append(found, c)
				}
			}
			switch len(found) {
			case 0:
				return fmt.Errorf("rust: %s: orphan module `%s` declared in %s: no %s; add the source or remove the declaration",
					root, m.Name, file, strings.Join(candidates, " or "))
			case 1:
				childDir := path.Dir(found[0])
				if path.Base(found[0]) != "mod.rs" {
					childDir = strings.TrimSuffix(found[0], ".rs")
				}
				if err := visit(found[0], childDir, inheritedTest || m.CfgTest, append(stack, file)); err != nil {
					return err
				}
			default:
				return fmt.Errorf("rust: %s: ambiguous module `%s` declared in %s: both %s exist; keep exactly one",
					root, m.Name, file, strings.Join(found, " and "))
			}
		}
		return nil
	}
	if err := visit(root, path.Dir(root), false, nil); err != nil {
		return nil, err
	}
	return owned, nil
}

// joinRel joins a module path relative to its declaring file's directory.
// A `#[path]` starting with `/` is workspace-relative; anything else is
// relative to the declaring file.
func joinRel(fileDir, rel string) string {
	if strings.HasPrefix(rel, "/") {
		return strings.TrimPrefix(path.Clean(rel), "/")
	}
	if fileDir == "" || fileDir == "." {
		return path.Clean(rel)
	}
	return path.Clean(fileDir + "/" + rel)
}
