// Dependency resolution, import validation, and rule attributes for the dx Rust Gazelle extension.

package rust

import (
	"fmt"
	"path"
	"sort"
	"strings"

	"github.com/bazel-contrib/bazel-gazelle/v2/label"
	"github.com/bazel-contrib/bazel-gazelle/v2/rule"
	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/repo"
	"github.com/bazelbuild/bazel-gazelle/resolve"
	bzl "github.com/bazelbuild/buildtools/build"
)

// resolveExtName is upstream Gazelle's private resolve configuration key,
// pinned through MODULE.bazel (gazelle 0.52.2). It is referenced literally
// only to probe for user overrides so validation stays silent-safe when the
// resolve configurer never ran (unit tests with bare configs); the real
// interpretation of mappings always goes through FindRuleWithOverride.
const resolveExtName = "_resolve"

func lookupOverride(c *config.Config, name string) (label.Label, bool) {
	var zero label.Label
	if c == nil {
		return zero, false
	}
	if _, ok := c.Exts[resolveExtName]; !ok {
		return zero, false
	}
	return resolve.FindRuleWithOverride(c, resolve.ImportSpec{Lang: languageName, Imp: name}, languageName)
}

// resolveImportOverride reports whether an import has an exact resolve
// mapping, failing when the mapping collides with an ignore directive.
func resolveImportOverride(c *config.Config, name string) (bool, error) {
	if _, ok := lookupOverride(c, name); ok {
		if ignore := matchingIgnore(c, name); ignore != nil {
			return false, fmt.Errorf("import %q has both an exact resolve mapping and an ignore directive", name)
		}
		return true, nil
	}
	return false, nil
}

func validateCargoImports(c *config.Config, manifest *cargoManifest, kind string, imports targetImports) error {
	for _, name := range imports.production {
		mapped, err := resolveImportOverride(c, name)
		if err != nil {
			return err
		}
		if mapped {
			continue
		}
		_, normal := manifest.normalDeps[name]
		_, dev := manifest.devDeps[name]
		if !normal && !(kind == testKind && dev) && name != strings.ReplaceAll(manifest.packageName, "-", "_") {
			return fmt.Errorf("unresolved production import %q; declare it in [dependencies] or add an exact mapping", name)
		}
	}
	for _, name := range imports.test {
		mapped, err := resolveImportOverride(c, name)
		if err != nil {
			return err
		}
		if mapped {
			continue
		}
		_, normal := manifest.normalDeps[name]
		_, dev := manifest.devDeps[name]
		if !normal && !dev {
			return fmt.Errorf("unresolved test import %q; declare it in [dev-dependencies] or add an exact mapping", name)
		}
	}
	return nil
}

// validateExampleImports checks example and bench imports: production
// imports may come from [dependencies] or [dev-dependencies] (Cargo links
// dev-dependencies into examples, benches, and tests), test-scoped imports
// from either as well.
func validateExampleImports(c *config.Config, manifest *cargoManifest, imports targetImports) error {
	for _, name := range imports.production {
		mapped, err := resolveImportOverride(c, name)
		if err != nil {
			return err
		}
		if mapped {
			continue
		}
		_, normal := manifest.normalDeps[name]
		_, dev := manifest.devDeps[name]
		if !normal && !dev && name != strings.ReplaceAll(manifest.packageName, "-", "_") {
			return fmt.Errorf("unresolved production import %q; declare it in [dependencies] or [dev-dependencies] or add an exact mapping", name)
		}
	}
	for _, name := range imports.test {
		mapped, err := resolveImportOverride(c, name)
		if err != nil {
			return err
		}
		if mapped {
			continue
		}
		_, normal := manifest.normalDeps[name]
		_, dev := manifest.devDeps[name]
		if !normal && !dev {
			return fmt.Errorf("unresolved test import %q; declare it in [dev-dependencies] or add an exact mapping", name)
		}
	}
	return nil
}

// validateBuildImports checks build-script imports: the script sees only
// [build-dependencies], never normal or dev dependencies and never its
// own crate (a script depending on its crate would cycle).
func validateBuildImports(c *config.Config, manifest *cargoManifest, imports targetImports) error {
	check := func(names []string, scope string) error {
		for _, name := range names {
			mapped, err := resolveImportOverride(c, name)
			if err != nil {
				return err
			}
			if mapped {
				continue
			}
			if _, ok := manifest.buildDeps[name]; !ok {
				return fmt.Errorf("unresolved %s import %q; declare it in [build-dependencies] or add an exact mapping", scope, name)
			}
		}
		return nil
	}
	if err := check(imports.production, "production"); err != nil {
		return err
	}
	return check(imports.test, "test")
}

func localCargoImports(c *config.Config, manifest *cargoManifest, imports targetImports, includeDev bool) targetImports {
	var result targetImports
	for _, name := range imports.production {
		if dep, ok := manifest.normalDeps[name]; ok && !dep.external {
			result.production = append(result.production, name)
		} else if includeDev {
			// Tests, examples, and benches link [dev-dependencies] in
			// every code position, including non-test ones: detection
			// records where the use item sits, not which scope Cargo
			// links it from.
			if dep, ok := manifest.devDeps[name]; ok && !dep.external {
				result.production = append(result.production, name)
			} else if _, ok := lookupOverride(c, name); ok {
				result.production = append(result.production, name)
			}
		} else if _, ok := lookupOverride(c, name); ok {
			result.production = append(result.production, name)
		}
	}
	if includeDev {
		for _, name := range imports.test {
			if dep, normal := manifest.normalDeps[name]; normal && !dep.external {
				result.test = append(result.test, name)
			} else if dep, dev := manifest.devDeps[name]; dev && !dep.external {
				result.test = append(result.test, name)
			} else if _, ok := lookupOverride(c, name); ok {
				result.test = append(result.test, name)
			}
		}
	}
	appendMirrorPaths(&result, manifest.normalDeps)
	if includeDev {
		appendMirrorPaths(&result, manifest.devDeps)
	}
	return result
}

// appendMirrorPaths links every declared first-party path dependency from
// the given scopes whether or not any use item names it. The set is
// deduplicated and sorted so resolution stays deterministic.
func appendMirrorPaths(result *targetImports, scopes ...map[string]cargoDependency) {
	seen := make(map[string]bool, len(result.mirrorPaths))
	for _, name := range result.mirrorPaths {
		seen[name] = true
	}
	for _, scope := range scopes {
		for name, dep := range scope {
			if !dep.external && dep.depPath != "" && !seen[name] {
				seen[name] = true
				result.mirrorPaths = append(result.mirrorPaths, name)
			}
		}
	}
	sort.Strings(result.mirrorPaths)
}

// localCargoExampleImports collects the first-party labels an example or
// bench rule resolves: path dependencies from [dependencies] and
// [dev-dependencies] alike (examples and benches link both). External
// dependencies resolve through the generated crate_deps call, never here.
func localCargoExampleImports(c *config.Config, manifest *cargoManifest, imports targetImports) targetImports {
	var result targetImports
	for _, name := range imports.production {
		if dep, ok := manifest.normalDeps[name]; ok && !dep.external {
			result.production = append(result.production, name)
		} else if dep, ok := manifest.devDeps[name]; ok && !dep.external {
			result.production = append(result.production, name)
		} else if _, ok := lookupOverride(c, name); ok {
			result.production = append(result.production, name)
		}
	}
	appendMirrorPaths(&result, manifest.normalDeps, manifest.devDeps)
	return result
}

// localCargoBuildImports collects the first-party labels a build-script
// rule resolves: path dependencies from [build-dependencies] only.
func localCargoBuildImports(c *config.Config, manifest *cargoManifest, imports targetImports) targetImports {
	var result targetImports
	for _, name := range imports.production {
		if dep, ok := manifest.buildDeps[name]; ok && !dep.external {
			result.production = append(result.production, name)
		} else if _, ok := lookupOverride(c, name); ok {
			result.production = append(result.production, name)
		}
	}
	for _, name := range imports.test {
		if dep, ok := manifest.buildDeps[name]; ok && !dep.external {
			result.test = append(result.test, name)
		} else if _, ok := lookupOverride(c, name); ok {
			result.test = append(result.test, name)
		}
	}
	appendMirrorPaths(&result, manifest.buildDeps)
	return result
}

func setCargoAttrs(r *rule.Rule, packagePath string, manifest *cargoManifest, imports targetImports, includeDev bool) {
	// crate_universe keys its maps by parent dir + Cargo package name
	// (e.g. cli/dx_output for //cli/output), not by Bazel package path.
	packageName := crateUniversePackage(packagePath, manifest)
	// Cargo links every declared dependency into every target of the package,
	// including path-only uses (`anyhow::Result`, `libc::c_int`) the use-path
	// parser never sees. Mirror that: deps carry all declared externals
	// (original dashed spelling for crate_universe lookup) plus detected
	// externals resolve to their declared label. First-party labels still
	// come from detected imports via Resolve.
	seen := make(map[string]bool)
	var external []string
	add := func(key string, dep cargoDependency) {
		if !dep.external {
			return
		}
		label := dep.label
		if label == "" {
			label = key
		}
		if !seen[label] {
			seen[label] = true
			external = append(external, label)
		}
	}
	names := append([]string{}, imports.production...)
	if includeDev {
		names = append(names, imports.test...)
	}
	for _, name := range names {
		if dep, ok := manifest.normalDeps[name]; ok {
			add(name, dep)
			continue
		}
		if includeDev {
			if dep, ok := manifest.devDeps[name]; ok {
				add(name, dep)
			}
		}
	}
	for key, dep := range manifest.normalDeps {
		add(key, dep)
	}
	if includeDev {
		for key, dep := range manifest.devDeps {
			add(key, dep)
		}
	}
	names = external
	sort.Strings(names)
	if len(names) > 0 {
		r.SetAttr("deps", crateDepsCall{names: names, packageName: packageName})
	}
	r.SetAttr("aliases", cargoCall("aliases", packageName, includeDev))
}

// setScriptAttrs sets the dependency attributes of a generated
// cargo_build_script rule: deps carry all declared external build
// dependencies (original dashed spelling for crate_universe lookup, which
// flattens the build maps into crate_deps) and aliases selects the build
// maps, so the script sees exactly [build-dependencies]. First-party
// labels still come from detected imports via Resolve.
func setScriptAttrs(r *rule.Rule, packagePath string, manifest *cargoManifest) {
	packageName := crateUniversePackage(packagePath, manifest)
	seen := make(map[string]bool)
	var external []string
	for key, dep := range manifest.buildDeps {
		if !dep.external {
			continue
		}
		label := dep.label
		if label == "" {
			label = key
		}
		if !seen[label] {
			seen[label] = true
			external = append(external, label)
		}
	}
	sort.Strings(external)
	if len(external) > 0 {
		r.SetAttr("deps", crateDepsCall{names: external, packageName: packageName})
	}
	r.SetAttr("aliases", cargoBuildCall(packageName))
}

// crateUniversePackage returns the crate_universe map key for a manifest:
// the parent Bazel directory joined with the Cargo package name. A nil or
// nameless manifest falls back to the Bazel path; a root-level package has
// no parent, so the Cargo name alone is the best guess.
func crateUniversePackage(packagePath string, manifest *cargoManifest) string {
	if manifest == nil || manifest.packageName == "" {
		return packagePath
	}
	if dir := path.Dir(packagePath); dir != "." && dir != "" {
		return dir + "/" + manifest.packageName
	}
	return manifest.packageName
}

func (l *rustLang) Resolve(c *config.Config, ix *resolve.RuleIndex, _ *repo.RemoteCache, r *rule.Rule, raw interface{}, from label.Label) {
	imports, ok := raw.(targetImports)
	if !ok {
		return
	}
	all := append([]string{}, imports.production...)
	if r.Kind() == testKind {
		all = append(all, imports.test...)
	}
	deps := make(map[string]bool)
	for _, name := range all {
		spec := resolve.ImportSpec{Lang: languageName, Imp: name}
		if override, found := resolve.FindRuleWithOverride(c, spec, languageName); found {
			if ignore := matchingIgnore(c, name); ignore != nil {
				ignore.used = true
				l.fail("rust: %s: import %q has both an exact resolve mapping and ignore", from, name)
				continue
			}
			deps[override.Rel(from.Repo, from.Pkg).String()] = true
			continue
		}
		matches := ix.FindRulesByImportWithConfig(c, spec, languageName)
		switch len(matches) {
		case 1:
			if matches[0].Label != from {
				deps[matches[0].Label.Rel(from.Repo, from.Pkg).String()] = true
			}
		case 0:
			if ignore := matchingIgnore(c, name); ignore != nil {
				ignore.used = true
				continue
			}
			l.fail("rust: %s: unresolved import %q; add a local crate, Cargo mapping, or exact # gazelle:resolve", from, name)
		default:
			l.fail("rust: %s: ambiguous import %q resolves to %s", from, name, formatMatches(matches))
		}
	}
	// Declared-but-undetected path dependencies mirror Cargo's linking
	// without detection evidence: overrides and unique index matches
	// become edges, while misses, ambiguities, and ignored names stay
	// silent. A genuinely used dep that detection missed still fails at
	// rustc with a precise error; failing closed here would instead
	// break legal trees whose declared deps this target never touches.
	for _, name := range imports.mirrorPaths {
		if ignore := matchingIgnore(c, name); ignore != nil {
			ignore.used = true
			continue
		}
		spec := resolve.ImportSpec{Lang: languageName, Imp: name}
		if override, found := resolve.FindRuleWithOverride(c, spec, languageName); found {
			deps[override.Rel(from.Repo, from.Pkg).String()] = true
			continue
		}
		if matches := ix.FindRulesByImportWithConfig(c, spec, languageName); len(matches) == 1 && matches[0].Label != from {
			deps[matches[0].Label.Rel(from.Repo, from.Pkg).String()] = true
		}
	}
	if imports.siblingLib != "" && imports.siblingLib != from.Name {
		addLocalDep(deps, from, imports.siblingLib)
	}
	if imports.scriptDep != "" && imports.scriptDep != from.Name {
		addLocalDep(deps, from, imports.scriptDep)
	}
	labels := make([]string, 0, len(deps))
	for dep := range deps {
		labels = append(labels, dep)
	}
	sort.Strings(labels)
	if len(labels) == 0 {
		return
	}
	// ResolveAttrs merge takes this output as final, so first-party labels
	// combine with (never replace) the generated crate_deps call; plain
	// label lists union in place.
	switch existing := r.Attr("deps"); existing.(type) {
	case nil:
		r.SetAttr("deps", labels)
	case *bzl.ListExpr:
		r.SetAttr("deps", unionStrings(r.AttrStrings("deps"), labels))
	default:
		r.SetAttr("deps", depsConcatExpr{base: existing, extra: labels})
	}
}

// addLocalDep records a same-package edge (sibling library, build script)
// in relative :name form. Import resolution can separately resolve the
// same target and render it absolute (//pkg:name) when the provider match
// carries a different repo appearance than the importing rule; both
// strings denote one target and Bazel rejects the duplicate, so drop the
// absolute form in favor of the relative one.
func addLocalDep(deps map[string]bool, from label.Label, name string) {
	for dep := range deps {
		if pkg, target, ok := splitDepLabel(dep, from); ok && pkg == from.Pkg && target == name {
			delete(deps, dep)
		}
	}
	deps[":"+name] = true
}

// splitDepLabel resolves a rendered dep string to its package and target
// names in from's repo context. It reports false for forms it cannot
// cheaply classify (which callers keep untouched).
func splitDepLabel(dep string, from label.Label) (string, string, bool) {
	rest := dep
	if strings.HasPrefix(rest, "@") {
		repo, after, found := strings.Cut(rest[1:], "//")
		if !found || (repo != "" && repo != from.Repo) {
			return "", "", false
		}
		rest = "//" + after
	}
	if strings.HasPrefix(rest, ":") {
		return from.Pkg, rest[1:], true
	}
	pkgTarget := strings.TrimPrefix(rest, "//")
	pkg, target, found := strings.Cut(pkgTarget, ":")
	if !found {
		if i := strings.LastIndex(pkg, "/"); i >= 0 {
			return pkg, pkg[i+1:], true
		}
		return pkg, pkg, true
	}
	return pkg, target, true
}

func unionStrings(a, b []string) []string {
	seen := make(map[string]bool, len(a)+len(b))
	var out []string
	for _, s := range append(append([]string{}, a...), b...) {
		if !seen[s] {
			seen[s] = true
			out = append(out, s)
		}
	}
	sort.Strings(out)
	return out
}

func matchingIgnore(c *config.Config, name string) *ignoreEntry {
	raw, ok := c.Exts[languageName]
	if !ok {
		return nil
	}
	for i := len(raw.(*rustConfig).ignores) - 1; i >= 0; i-- {
		if entry := raw.(*rustConfig).ignores[i]; entry.value == name {
			return entry
		}
	}
	return nil
}

func formatMatches(matches []resolve.FindResult) string {
	labels := make([]string, 0, len(matches))
	for _, match := range matches {
		labels = append(labels, match.Label.String())
	}
	sort.Strings(labels)
	return fmt.Sprintf("[%s]", strings.Join(labels, ", "))
}
