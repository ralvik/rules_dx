package rust

import (
	"fmt"
	"path"
	"strconv"
	"strings"
)

type cargoTarget struct {
	kind string
	// name is the Bazel target name, disambiguated when a library shares
	// its crate name with a binary (see disambiguateLibBin).
	name string
	// crateName is the Rust crate name; empty means name.
	crateName string
	path      string
	harness   bool
}

// crate reports the Rust crate name, which only differs from the Bazel
// target name after lib/bin disambiguation.
func (t cargoTarget) crate() string {
	if t.crateName != "" {
		return t.crateName
	}
	return t.name
}

type cargoManifest struct {
	packageName string
	edition     string
	targets     []cargoTarget
	normalDeps  map[string]cargoDependency
	devDeps     map[string]cargoDependency
}

func (m *cargoManifest) withImplicitTargets(files map[string]bool, packagePath string) error {
	explicitPaths := make(map[string]bool)
	hasLib := false
	hasDefaultBin := false
	for _, target := range m.targets {
		explicitPaths[target.path] = true
		if target.kind == libraryKind {
			hasLib = true
		}
		if target.kind == binaryKind && target.path == "src/main.rs" {
			hasDefaultBin = true
		}
	}
	if !hasLib && files[path.Join(packagePath, "src/lib.rs")] {
		m.targets = append(m.targets, cargoTarget{kind: libraryKind, name: m.packageName, path: "src/lib.rs", harness: true})
	}
	if !hasDefaultBin && !explicitPaths["src/main.rs"] && files[path.Join(packagePath, "src/main.rs")] {
		m.targets = append(m.targets, cargoTarget{kind: binaryKind, name: m.packageName, path: "src/main.rs", harness: true})
	}
	prefix := path.Join(packagePath, "tests") + "/"
	for file := range files {
		rel := strings.TrimPrefix(file, prefix)
		if !strings.HasPrefix(file, prefix) || rel == "" || strings.Contains(rel, "/") || !strings.HasSuffix(rel, ".rs") {
			continue
		}
		targetPath := "tests/" + rel
		if explicitPaths[targetPath] {
			continue
		}
		name, err := Normalize(strings.TrimSuffix(rel, ".rs"))
		if err != nil {
			return fmt.Errorf("rust: Cargo integration-test root %s: %w", file, err)
		}
		m.targets = append(m.targets, cargoTarget{kind: testKind, name: IntegrationTestName(name), path: targetPath, harness: true})
	}
	m.disambiguateLibBin()
	return m.validateTargetClaims()
}

// disambiguateLibBin renames a library target sharing its name with a binary
// target to `<name>_lib`, preserving the Rust crate name. This converges
// generated rules with the hand-written convention (library `X_lib`,
// binary `X`) so same-kind merge takes over the handwritten target instead
// of failing on a cross-kind claim. Residual collisions still fail loudly
// in validateTargetClaims.
func (m *cargoManifest) disambiguateLibBin() {
	bins := make(map[string]bool)
	for _, target := range m.targets {
		if target.kind == binaryKind {
			bins[target.name] = true
		}
	}
	for i := range m.targets {
		target := &m.targets[i]
		if target.kind == libraryKind && bins[target.name] {
			if target.crateName == "" {
				target.crateName = target.name
			}
			target.name += "_lib"
		}
	}
}

func (m *cargoManifest) validateTargetClaims() error {
	seenNames := make(map[string]string)
	seenPaths := make(map[string]string)
	for _, target := range m.targets {
		if previous, ok := seenNames[target.name]; ok {
			return fmt.Errorf("Cargo target name %q is claimed by %s and %s; use distinct authoritative names", target.name, previous, target.kind)
		}
		if previous, ok := seenPaths[target.path]; ok {
			return fmt.Errorf("Cargo target path %q is claimed by %s and %s; one source may have only one owner", target.path, previous, target.name)
		}
		seenNames[target.name] = target.kind
		seenPaths[target.path] = target.name
	}
	return nil
}

type cargoDependency struct {
	external bool
	// label is the original Cargo.toml spelling (dashes preserved) for
	// crate_universe lookup; the map key is the normalized Rust ident.
	label string
}

func parseCargoManifest(manifestPath string, content []byte) (*cargoManifest, error) {
	manifest := &cargoManifest{
		edition:    "2021",
		normalDeps: make(map[string]cargoDependency),
		devDeps:    make(map[string]cargoDependency),
	}
	section := ""
	current := -1
	hasLib := false
	for number, raw := range strings.Split(string(content), "\n") {
		line := strings.TrimSpace(stripTomlComment(raw))
		if line == "" {
			continue
		}
		if strings.HasPrefix(line, "[") {
			switch line {
			case "[package]", "[lib]":
				section = strings.Trim(line, "[]")
				if section == "lib" {
					hasLib = true
					manifest.targets = append(manifest.targets, cargoTarget{kind: libraryKind, harness: true})
					current = len(manifest.targets) - 1
				} else {
					current = -1
				}
			case "[[bin]]", "[[test]]":
				section = strings.Trim(line, "[]")
				kind := binaryKind
				if section == "test" {
					kind = testKind
				}
				manifest.targets = append(manifest.targets, cargoTarget{kind: kind, harness: true})
				current = len(manifest.targets) - 1
			default:
				section = strings.Trim(line, "[]")
				current = -1
			}
			continue
		}
		parts := strings.SplitN(line, "=", 2)
		if len(parts) != 2 {
			return nil, fmt.Errorf("rust: %s:%d: malformed Cargo assignment", manifestPath, number+1)
		}
		key, value := strings.TrimSpace(parts[0]), strings.TrimSpace(parts[1])
		switch section {
		case "package":
			if key == "name" || key == "edition" {
				parsed, err := tomlString(value)
				if err != nil {
					return nil, fmt.Errorf("rust: %s:%d: %s: %v", manifestPath, number+1, key, err)
				}
				if key == "name" {
					manifest.packageName = parsed
				} else {
					manifest.edition = parsed
				}
			}
		case "lib", "bin", "test":
			switch key {
			case "name", "path":
				parsed, err := tomlString(value)
				if err != nil {
					return nil, fmt.Errorf("rust: %s:%d: %s: %v", manifestPath, number+1, key, err)
				}
				if key == "name" {
					manifest.targets[current].name = parsed
				} else {
					manifest.targets[current].path = path.Clean(parsed)
				}
			case "harness":
				parsed, err := strconv.ParseBool(value)
				if err != nil {
					return nil, fmt.Errorf("rust: %s:%d: harness must be true or false", manifestPath, number+1)
				}
				manifest.targets[current].harness = parsed
			case "required-features", "crate-type", "proc-macro":
				return nil, fmt.Errorf("rust: %s:%d: unsupported Cargo target field %q; use a kept handwritten target", manifestPath, number+1, key)
			}
		case "dependencies", "dev-dependencies":
			name := strings.ReplaceAll(key, "-", "_")
			dependency := cargoDependency{external: !strings.Contains(value, "path"), label: key}
			if section == "dependencies" {
				manifest.normalDeps[name] = dependency
			} else {
				manifest.devDeps[name] = dependency
			}
		}
	}
	if manifest.packageName == "" {
		return nil, fmt.Errorf("rust: %s: missing [package].name", manifestPath)
	}
	if !hasLib {
		manifest.targets = append(manifest.targets, cargoTarget{kind: libraryKind, name: manifest.packageName, path: "src/lib.rs", harness: true})
	}
	for i := range manifest.targets {
		target := &manifest.targets[i]
		if target.name == "" {
			target.name = manifest.packageName
		}
		if target.path == "" {
			switch target.kind {
			case libraryKind:
				target.path = "src/lib.rs"
			case binaryKind:
				target.path = "src/main.rs"
			case testKind:
				target.path = "tests/" + target.name + ".rs"
			}
		}
		normalized, err := Normalize(target.name)
		if err != nil {
			return nil, fmt.Errorf("rust: %s: Cargo target %q: %v", manifestPath, target.name, err)
		}
		target.name = normalized
	}
	manifest.disambiguateLibBin()
	if err := manifest.validateTargetClaims(); err != nil {
		return nil, fmt.Errorf("rust: %s: %v", manifestPath, err)
	}
	return manifest, nil
}

func stripTomlComment(line string) string {
	inString := false
	escaped := false
	for i, char := range line {
		if inString {
			if escaped {
				escaped = false
			} else if char == '\\' {
				escaped = true
			} else if char == '"' {
				inString = false
			}
		} else if char == '"' {
			inString = true
		} else if char == '#' {
			return line[:i]
		}
	}
	return line
}

func tomlString(value string) (string, error) {
	if len(value) < 2 || value[0] != '"' || value[len(value)-1] != '"' {
		return "", fmt.Errorf("expected a basic quoted string")
	}
	parsed, err := strconv.Unquote(value)
	if err != nil {
		return "", err
	}
	return parsed, nil
}
