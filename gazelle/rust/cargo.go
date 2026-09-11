package rust

import (
	"fmt"
	"os"
	"path"
	"strconv"
	"strings"
)

const (
	// exampleKind and benchKind are manifest-internal target kinds for
	// explicitly declared [[example]] and [[bench]] targets; emission
	// maps both to ordinary dx_rust_binary rules with affixed names.
	exampleKind = "example"
	benchKind   = "bench"
)

type cargoTarget struct {
	kind string // name is the Bazel target name, disambiguated when a library shares
	// its crate name with a binary (see disambiguateLibBin).
	name string
	// logical is the authoritative Cargo target name. It equals name
	// except for examples and benches, which gain the `_example` and
	// `_bench` affixes in name while logical keeps the Cargo name for
	// example-test wrappers and diagnostics.
	logical string
	// crateName is the Rust crate name; empty means name.
	crateName string
	path      string
	// harness selects the libtest harness for [[test]] targets: true
	// (default) generates a libtest wrapper, false marks a custom-harness
	// executable. exampleTest mirrors the flag for [[example]] targets
	// declared with `test = true`: an example stays an ordinary binary
	// but additionally gains a libtest wrapper. Benches never gain
	// wrappers.
	harness     bool
	exampleTest bool
	// requiredFeatures records a `required-features` list verbatim; the
	// manifest rejects it after parsing with the target name for context.
	requiredFeatures []string
	// crateTypes records a [lib] `crate-type` list; procMacro records
	// `[lib] proc-macro = true`. resolveLibFlavor folds both into flavor.
	crateTypes []string
	procMacro  bool
	// flavor is the resolved library shape for libraryKind targets: ""
	// (ordinary rlib), "proc-macro", "cdylib", or "staticlib".
	flavor string
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
	// version is the [package] version; empty when undeclared. It feeds
	// the generated build-script rule and single-version checks on
	// first-party path dependencies.
	version    string
	targets    []cargoTarget
	normalDeps map[string]cargoDependency
	devDeps    map[string]cargoDependency
	// buildDeps holds [build-dependencies]: only the build script sees
	// them, never lib/bin/test/example/bench targets.
	buildDeps map[string]cargoDependency
	// build is nil when [package] declares no build key: build scripts
	// stay explicit and an undeclared build.rs fails closed.
	build *cargoBuild
}

// cargoBuild describes one active Cargo build script: the manifest-relative
// cleaned script path, or disabled when `build = false`.
type cargoBuild struct {
	path     string
	disabled bool
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
		m.targets = append(m.targets, cargoTarget{kind: libraryKind, name: m.packageName, logical: m.packageName, path: "src/lib.rs", harness: true})
	}
	if !hasDefaultBin && !explicitPaths["src/main.rs"] && files[path.Join(packagePath, "src/main.rs")] {
		m.targets = append(m.targets, cargoTarget{kind: binaryKind, name: m.packageName, logical: m.packageName, path: "src/main.rs", harness: true})
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
		m.targets = append(m.targets, cargoTarget{kind: testKind, name: IntegrationTestName(name), logical: IntegrationTestName(name), path: targetPath, harness: true})
	}
	// Build scripts stay explicit: an undeclared build.rs fails closed
	// instead of silently diverging from Cargo, which would run it.
	// Declare `build = "build.rs"` to generate the script rule or
	// `build = false` to keep the file inert.
	if m.build == nil && files[path.Join(packagePath, "build.rs")] {
		return fmt.Errorf("rust: %s/build.rs exists without a [package] build key; declare build = \"build.rs\" or build = false", packagePath)
	}
	m.disambiguateLibBin()
	return m.validateTargetClaims()
}

// disambiguateLibBin renames a library target sharing its name with a binary
// target to `<name>_lib`, preserving the Rust crate name. This converges
// generated rules with the hand-written convention (library `X_lib`,
// binary `X`) so same-kind merge takes over the handwritten target instead
// of failing on a cross-kind claim. Examples and benches emit binaries, so
// their affixed names join the binary set. Residual collisions still fail
// loudly in validateTargetClaims.
func (m *cargoManifest) disambiguateLibBin() {
	bins := make(map[string]bool)
	for _, target := range m.targets {
		if target.kind == binaryKind || target.kind == exampleKind || target.kind == benchKind {
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
		if len(target.requiredFeatures) > 0 {
			return fmt.Errorf("Cargo target %s(%s) requires features %q; required-features stay opt-in: keep a handwritten testonly target selecting them", target.logical, target.kind, strings.Join(target.requiredFeatures, ", "))
		}
		if target.kind == benchKind && target.harness {
			return fmt.Errorf("Cargo bench %s leaves the libtest harness enabled; benchmarks generate only with harness = false, otherwise keep a handwritten target", target.logical)
		}
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
	// version is the declared version requirement when one rides along
	// the dependency value (e.g. a path dependency with `version`);
	// empty means unconstrained.
	version string
	// depPath is the declared `path` for path dependencies, relative to
	// the declaring manifest directory; empty for external dependencies.
	depPath string
}

func parseCargoManifest(manifestPath string, content []byte) (*cargoManifest, error) {
	manifest := &cargoManifest{
		edition:    "2021",
		normalDeps: make(map[string]cargoDependency),
		devDeps:    make(map[string]cargoDependency),
		buildDeps:  make(map[string]cargoDependency),
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
			case "[[bin]]", "[[test]]", "[[example]]", "[[bench]]":
				section = strings.Trim(line, "[]")
				kind := binaryKind
				switch section {
				case "test":
					kind = testKind
				case "example":
					kind = exampleKind
				case "bench":
					kind = benchKind
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
			switch key {
			case "name", "edition":
				parsed, err := tomlString(value)
				if err != nil {
					return nil, fmt.Errorf("rust: %s:%d: %s: %v", manifestPath, number+1, key, err)
				}
				if key == "name" {
					manifest.packageName = parsed
				} else {
					manifest.edition = parsed
				}
			case "version":
				parsed, err := tomlString(value)
				if err != nil {
					return nil, fmt.Errorf("rust: %s:%d: %s: %v", manifestPath, number+1, key, err)
				}
				manifest.version = parsed
			case "build":
				// `build = false` disables the script; `build = true`
				// selects the conventional build.rs; a string names the
				// script explicitly. Absence means no script: build
				// scripts stay explicit and an undeclared build.rs fails
				// closed in withImplicitTargets.
				switch value {
				case "false":
					manifest.build = &cargoBuild{disabled: true}
				case "true":
					manifest.build = &cargoBuild{path: "build.rs"}
				default:
					parsed, err := tomlString(value)
					if err != nil {
						return nil, fmt.Errorf("rust: %s:%d: build must be a path, true, or false", manifestPath, number+1)
					}
					manifest.build = &cargoBuild{path: path.Clean(parsed)}
				}
			}
		case "lib", "bin", "test", "example", "bench":
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
				if section == "example" {
					return nil, fmt.Errorf("rust: %s:%d: [[example]] has no harness key; declare test = true for a libtest wrapper or keep the example handwritten", manifestPath, number+1)
				}
				manifest.targets[current].harness = parsed
			case "test":
				// Accepted inert: [[bin]] test=false, [[test]] test=false,
				// and [lib] test only narrow what `cargo test` runs; the
				// generated unit-test wrappers are a superset that stays
				// correct either way. [[example]] test=true is the one
				// test key with emission meaning (see below).
				if section == "example" {
					parsed, err := strconv.ParseBool(value)
					if err != nil {
						return nil, fmt.Errorf("rust: %s:%d: test must be true or false", manifestPath, number+1)
					}
					manifest.targets[current].exampleTest = parsed
					break
				}
				if _, err := strconv.ParseBool(value); err != nil {
					return nil, fmt.Errorf("rust: %s:%d: test must be true or false", manifestPath, number+1)
				}
			case "bench":
				// Accepted inert: the generated binaries always build, so
				// they satisfy `bench = true` and `bench = false` alike.
				// Benches never gain test wrappers regardless of this key.
				if _, err := strconv.ParseBool(value); err != nil {
					return nil, fmt.Errorf("rust: %s:%d: bench must be true or false", manifestPath, number+1)
				}
			case "crate-type":
				if section != "lib" {
					return nil, fmt.Errorf("rust: %s:%d: unsupported Cargo target field %q; use a kept handwritten target", manifestPath, number+1, key)
				}
				parsed, err := tomlStringList(value)
				if err != nil {
					return nil, fmt.Errorf("rust: %s:%d: crate-type: %v", manifestPath, number+1, err)
				}
				manifest.targets[current].crateTypes = parsed
			case "proc-macro":
				if section != "lib" {
					return nil, fmt.Errorf("rust: %s:%d: unsupported Cargo target field %q; use a kept handwritten target", manifestPath, number+1, key)
				}
				parsed, err := strconv.ParseBool(value)
				if err != nil {
					return nil, fmt.Errorf("rust: %s:%d: proc-macro must be true or false", manifestPath, number+1)
				}
				manifest.targets[current].procMacro = parsed
			case "required-features":
				// Recorded, not rejected here: the post-parse check
				// reports the target name and manifest for context.
				parsed, err := tomlStringList(value)
				if err != nil {
					return nil, fmt.Errorf("rust: %s:%d: required-features: %v", manifestPath, number+1, err)
				}
				manifest.targets[current].requiredFeatures = parsed
			}
		case "dependencies", "dev-dependencies", "build-dependencies":
			name := strings.ReplaceAll(key, "-", "_")
			dependency := cargoDependency{
				external: !strings.Contains(value, "path"),
				label:    key,
				version:  tomlInlineField(value, "version"),
			}
			if !dependency.external {
				dependency.depPath = tomlInlineField(value, "path")
			}
			switch section {
			case "dependencies":
				manifest.normalDeps[name] = dependency
			case "dev-dependencies":
				manifest.devDeps[name] = dependency
			default:
				manifest.buildDeps[name] = dependency
			}
		}
	}
	if manifest.packageName == "" {
		return nil, fmt.Errorf("rust: %s: missing [package].name", manifestPath)
	}
	// The package name feeds BuildScriptName's normalizer: reject an
	// un-normalizable name here instead of panicking at emission.
	if _, err := Normalize(manifest.packageName); err != nil {
		return nil, fmt.Errorf("rust: %s: [package].name %q: %v", manifestPath, manifest.packageName, err)
	}
	if !hasLib {
		manifest.targets = append(manifest.targets, cargoTarget{kind: libraryKind, name: manifest.packageName, path: "src/lib.rs", harness: true})
	}
	for i := range manifest.targets {
		target := &manifest.targets[i]
		if target.name == "" && target.path == "" && (target.kind == exampleKind || target.kind == benchKind) {
			return nil, fmt.Errorf("rust: %s: %s target needs a name or path", manifestPath, target.kind)
		}
		if target.name == "" {
			if target.path != "" && (target.kind == exampleKind || target.kind == benchKind) {
				// Cargo derives an example/bench name from the root file
				// stem when only a path is declared.
				target.name = strings.TrimSuffix(path.Base(target.path), ".rs")
			} else {
				target.name = manifest.packageName
			}
		}
		if target.path == "" {
			switch target.kind {
			case libraryKind:
				target.path = "src/lib.rs"
			case binaryKind:
				target.path = "src/main.rs"
			case testKind:
				target.path = "tests/" + target.name + ".rs"
			case exampleKind:
				target.path = "examples/" + target.name + ".rs"
			case benchKind:
				target.path = "benches/" + target.name + ".rs"
			}
		}
		// logical keeps the authoritative Cargo name for diagnostics and
		// example-test wrappers; name becomes the final Bazel name.
		target.logical = target.name
		normalized, err := Normalize(target.name)
		if err != nil {
			return nil, fmt.Errorf("rust: %s: Cargo target %q: %v", manifestPath, target.name, err)
		}
		switch target.kind {
		case exampleKind, benchKind:
			// Mirrors ExampleName/BenchName without panicking so the
			// manifest path stays in the error.
			target.name = normalized
			if target.kind == exampleKind {
				target.name += "_example"
			} else {
				target.name += "_bench"
			}
			target.crateName = normalized
		default:
			target.name = normalized
		}
	}
	if err := manifest.resolveLibFlavors(); err != nil {
		return nil, fmt.Errorf("rust: %s: %v", manifestPath, err)
	}
	manifest.disambiguateLibBin()
	if err := manifest.validateTargetClaims(); err != nil {
		return nil, fmt.Errorf("rust: %s: %v", manifestPath, err)
	}
	return manifest, nil
}

// resolveLibFlavors folds each library target's [lib] `crate-type` list
// and `proc-macro` flag into its flavor: "" (ordinary rlib), "proc-macro",
// "cdylib", or "staticlib". Anything that cannot map to one Bazel target
// fails with a handwritten-target pointer: multiple crate types for one
// source owner, dynamic dylib output, unknown type spellings, and
// proc-macro/crate-type conflicts.
func (m *cargoManifest) resolveLibFlavors() error {
	for i := range m.targets {
		target := &m.targets[i]
		if target.kind != libraryKind {
			continue
		}
		types := target.crateTypes
		conflict := fmt.Errorf("library %s sets proc-macro = true with crate-type = %q; declare one shape", target.logical, types)
		switch {
		case target.procMacro && len(types) == 0:
			target.flavor = "proc-macro"
		case len(types) == 0:
			target.flavor = ""
		case len(types) > 1:
			return fmt.Errorf("library %s declares multiple crate types %q; one source owner maps to one Bazel target, so keep a handwritten target", target.logical, types)
		default:
			switch types[0] {
			case "lib", "rlib":
				if target.procMacro {
					return conflict
				}
				target.flavor = ""
			case "proc-macro":
				target.flavor = "proc-macro"
			case "cdylib":
				if target.procMacro {
					return conflict
				}
				target.flavor = "cdylib"
			case "staticlib":
				if target.procMacro {
					return conflict
				}
				target.flavor = "staticlib"
			default:
				return fmt.Errorf("library %s declares unsupported crate-type %q; keep a handwritten target", target.logical, types[0])
			}
		}
	}
	return nil
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

// tomlStringList parses a single-line TOML string list such as
// `["cdylib", "rlib"]`. Multi-line lists stay unsupported, matching the
// line-oriented manifest reader.
func tomlStringList(value string) ([]string, error) {
	trimmed := strings.TrimSpace(value)
	if len(trimmed) < 2 || trimmed[0] != '[' || trimmed[len(trimmed)-1] != ']' {
		return nil, fmt.Errorf("expected a string list")
	}
	inner := strings.TrimSpace(trimmed[1 : len(trimmed)-1])
	if inner == "" {
		return nil, nil
	}
	var out []string
	for _, item := range strings.Split(inner, ",") {
		parsed, err := tomlString(strings.TrimSpace(item))
		if err != nil {
			return nil, err
		}
		out = append(out, parsed)
	}
	return out, nil
}

// tomlInlineField extracts one quoted string field from a single-line
// inline table value such as `{ version = "1.2", path = "../foo" }`.
// It returns "" when the field is absent or malformed. The key must stand
// alone (start of value, `{`, or `,` before it) so `myversion` never
// matches `version` and quoted occurrences never match.
func tomlInlineField(value, field string) string {
	search := value
	for len(search) > 0 {
		idx := strings.Index(search, field)
		if idx < 0 {
			return ""
		}
		before, after := search[:idx], search[idx+len(field):]
		beforeTrim := strings.TrimSpace(before)
		afterTrim := strings.TrimSpace(after)
		if strings.HasPrefix(afterTrim, "=") && (beforeTrim == "" || strings.HasSuffix(beforeTrim, "{") || strings.HasSuffix(beforeTrim, ",")) {
			rest := strings.TrimSpace(strings.TrimPrefix(afterTrim, "="))
			if len(rest) == 0 || rest[0] != '"' {
				return ""
			}
			for i := 1; i < len(rest); i++ {
				if rest[i] == '\\' {
					i++
					continue
				}
				if rest[i] == '"' {
					if parsed, err := strconv.Unquote(rest[:i+1]); err == nil {
						return parsed
					}
					return ""
				}
			}
			return ""
		}
		search = after
	}
	return ""
}

// scanPackageVersion returns the [package] version from manifest content
// without running the full strict parser, for reading provider versions
// across first-party path edges.
func scanPackageVersion(content []byte) string {
	section := ""
	for _, raw := range strings.Split(string(content), "\n") {
		line := strings.TrimSpace(stripTomlComment(raw))
		if line == "" {
			continue
		}
		if strings.HasPrefix(line, "[") {
			section = strings.Trim(line, "[]")
			continue
		}
		if section != "package" {
			continue
		}
		parts := strings.SplitN(line, "=", 2)
		if len(parts) != 2 || strings.TrimSpace(parts[0]) != "version" {
			continue
		}
		if version, err := tomlString(strings.TrimSpace(parts[1])); err == nil {
			return version
		}
	}
	return ""
}

// checkPathDepVersions enforces single-version resolution on first-party
// path edges that declare a version requirement: the provider's [package]
// version must satisfy the depender's requirement, else generation fails
// naming both manifests. Unconstrained edges (no version key) pass.
func checkPathDepVersions(packageDir, manifestPath string, manifest *cargoManifest) error {
	depMaps := []map[string]cargoDependency{manifest.normalDeps, manifest.devDeps, manifest.buildDeps}
	for _, depMap := range depMaps {
		for key, dep := range depMap {
			if dep.external || dep.version == "" || dep.depPath == "" {
				continue
			}
			provider := path.Join(packageDir, dep.depPath, "Cargo.toml")
			content, err := os.ReadFile(provider)
			if err != nil {
				return fmt.Errorf("path dependency %q points at %s without a readable Cargo.toml; fix the path", key, provider)
			}
			provided := scanPackageVersion(content)
			if provided == "" {
				return fmt.Errorf("path dependency %q at %s declares no [package] version while %s requires %q; version the provider or drop the requirement", key, provider, manifestPath, dep.version)
			}
			if !versionReqSatisfied(provided, dep.version) {
				return fmt.Errorf("path dependency %q provides version %s at %s, which does not satisfy %q required by %s", key, provided, provider, dep.version, manifestPath)
			}
		}
	}
	return nil
}

// versionReqSatisfied reports whether a provider version satisfies a Cargo
// version requirement: comma-separated clauses of caret (bare, `^`),
// tilde (`~`), exact (`=`), and ordered (`>=`, `<=`, `>`, `<`) comparisons
// over numeric semver triples. "*" and empty requirements always pass.
func versionReqSatisfied(version, req string) bool {
	req = strings.TrimSpace(req)
	if req == "" || req == "*" {
		return true
	}
	for _, clause := range strings.Split(req, ",") {
		if !evalVersionClause(version, strings.TrimSpace(clause)) {
			return false
		}
	}
	return true
}

func evalVersionClause(version, clause string) bool {
	bound := ""
	rest := clause
	for _, op := range []string{">=", "<=", ">", "<", "=", "^", "~"} {
		if strings.HasPrefix(clause, op) {
			bound = op
			rest = strings.TrimSpace(strings.TrimPrefix(clause, op))
			break
		}
	}
	current := parseSemver(version)
	want := parseSemver(rest)
	cmp := compareSemver(current, want)
	switch bound {
	case ">=":
		return cmp >= 0
	case "<=":
		return cmp <= 0
	case ">":
		return cmp > 0
	case "<":
		return cmp < 0
	case "=":
		return cmp == 0
	case "~":
		// ~1.2.3 means >=1.2.3, <1.3.0; ~1.2 and ~1 widen the cap.
		return cmp >= 0 && compareSemver(current, tildeUpper(want, rest)) < 0
	default:
		// Bare and ^ requirements are Cargo caret semantics.
		return cmp >= 0 && compareSemver(current, caretUpper(want, rest)) < 0
	}
}

// parseSemver reduces a version to its numeric triple, dropping any
// pre-release or build suffix. Missing components default to zero;
// unparseable components fail the comparison they feed.
func parseSemver(version string) [3]int {
	var out [3]int
	core := version
	if idx := strings.IndexAny(core, "-+"); idx >= 0 {
		core = core[:idx]
	}
	for i, part := range strings.Split(core, ".") {
		if i >= 3 {
			break
		}
		number, err := strconv.Atoi(strings.TrimSpace(part))
		if err != nil {
			return [3]int{-1, -1, -1}
		}
		out[i] = number
	}
	return out
}

func compareSemver(a, b [3]int) int {
	for i := 0; i < 3; i++ {
		if a[i] != b[i] {
			if a[i] < b[i] {
				return -1
			}
			return 1
		}
	}
	return 0
}

// caretUpper returns the exclusive upper bound of a caret requirement,
// honoring the precision the requirement was written with: ^1.2.3 caps at
// 2.0.0, ^0.2.3 at 0.3.0, ^0.0.3 at 0.0.4, ^0.2 at 0.3.0, ^0 at 1.0.0.
func caretUpper(want [3]int, rest string) [3]int {
	parts := strings.Split(rest, ".")
	switch {
	case want[0] > 0:
		return [3]int{want[0] + 1, 0, 0}
	case len(parts) == 1:
		return [3]int{1, 0, 0}
	case want[1] > 0:
		return [3]int{0, want[1] + 1, 0}
	default:
		return [3]int{0, 0, want[2] + 1}
	}
}

// tildeUpper returns the exclusive upper bound of a tilde requirement:
// ~1.2.3 and ~1.2 cap at 1.3.0, ~1 at 2.0.0.
func tildeUpper(want [3]int, rest string) [3]int {
	if len(strings.Split(rest, ".")) <= 1 {
		return [3]int{want[0] + 1, 0, 0}
	}
	return [3]int{want[0], want[1] + 1, 0}
}
