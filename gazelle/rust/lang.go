package rust

import (
	"context"
	"flag"
	"fmt"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"

	"github.com/bazel-contrib/bazel-gazelle/v2/label"
	"github.com/bazel-contrib/bazel-gazelle/v2/rule"
	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/repo"
	"github.com/bazelbuild/bazel-gazelle/resolve"
	bzl "github.com/bazelbuild/buildtools/build"
)

const (
	languageName = "rust"
	libraryKind  = "dx_rust_library"
	binaryKind   = "dx_rust_binary"
	testKind     = "dx_rust_test"
	// procMacroKind, sharedKind, and staticKind are the dx wrappers for
	// Cargo [lib] targets with proc-macro = true, crate-type =
	// ["cdylib"], and crate-type = ["staticlib"].
	procMacroKind = "dx_rust_proc_macro"
	sharedKind    = "dx_rust_shared_library"
	staticKind    = "dx_rust_static_library"
	// scriptKind is the upstream cargo_build_script wrapper macro (loaded
	// from @rules_rust//cargo), not a dx wrapper: the macro already owns
	// the script-binary/runfiles split and stays self-describing.
	scriptKind = "cargo_build_script"
)

var rustKinds = map[string]rule.KindInfo{
	libraryKind:   kindInfo(),
	binaryKind:    kindInfo(),
	testKind:      kindInfo(),
	procMacroKind: kindInfo(),
	sharedKind:    kindInfo(),
	staticKind:    kindInfo(),
	scriptKind:    kindInfo(),
}

func init() {
	// Managed native-config targets merge through the same file the
	// Rust rules live in, so their kinds register alongside.
	for kind, info := range nativeConfigKinds() {
		rustKinds[kind] = info
	}
}

func kindInfo() rule.KindInfo {
	return rule.KindInfo{
		MatchAttrs: []string{"crate_root"},
		NonEmptyAttrs: map[string]bool{
			"crate": true,
			"srcs":  true,
		},
		SubstituteAttrs: map[string]bool{"crate": true},
		MergeableAttrs: map[string]bool{
			"aliases":      true,
			"aspect_hints": true,
			"crate":        true,
			"crate_name":   true,
			"crate_root":   true,
			"srcs":         true,
		},
		ResolveAttrs: map[string]bool{"deps": true},
	}
}

type rustLang struct {
	language.BaseLang
	errors   []string
	ignores  []*ignoreEntry
	manifest *manifestRecorder
}

type rustConfig struct {
	ignores []*ignoreEntry
	// tools is the managed native-config tool set for the directory,
	// from the nearest dx_native_tools directive or inherited. Nil
	// means unconstrained: every managed tool.
	tools []string
}

type ignoreEntry struct {
	value string
	path  string
	used  bool
}

type targetImports struct {
	production []string
	test       []string
	// siblingLib is the Bazel name of the same-package library a binary
	// target links automatically (Cargo bins bind their sibling lib
	// without an import). Empty for non-binaries. It flows straight to
	// deps: validation and manifest indexing ignore it.
	siblingLib string
	// scriptDep is the Bazel name of the package's generated build-script
	// rule. Every crate rule in a package with an active script links it
	// (the upstream consumer pattern is a plain deps edge), except the
	// script rule itself. It flows straight to deps like siblingLib.
	scriptDep string
}

// NewLanguage returns the private first-party Rust Gazelle extension.
func NewLanguage() language.Language { return &rustLang{} }

func (l *rustLang) Before(context.Context) {
	l.errors = nil
	l.ignores = nil
	l.manifest = loadManifestRecorder()
	if l.manifest != nil {
		l.manifest.apparentLoads = l.ApparentLoads
	}
}

func (*rustLang) DoneGeneratingRules() {}

func (l *rustLang) RegisterFlags(*flag.FlagSet, string, *config.Config) {}

func (l *rustLang) CheckFlags(*flag.FlagSet, *config.Config) error { return nil }

func (*rustLang) KnownDirectives() []string { return []string{"dx_ignore_import", nativeToolsDirective} }

func (l *rustLang) Configure(c *config.Config, rel string, file *rule.File) {
	var inherited []*ignoreEntry
	tools := defaultNativeTools()
	if raw, ok := c.Exts[languageName]; ok {
		parent := raw.(*rustConfig)
		inherited = append(inherited, parent.ignores...)
		if parent.tools != nil {
			tools = parent.tools
		}
	}
	if file != nil {
		for _, directive := range file.Directives {
			switch directive.Key {
			case nativeToolsDirective:
				selected, err := parseNativeToolsDirective(directive.Value)
				if err != nil {
					l.fail("%v", err)
					continue
				}
				// The nearest directive wins: a deeper BUILD file
				// replaces the inherited set instead of unioning it.
				tools = selected
			case "dx_ignore_import":
				fields := strings.Fields(directive.Value)
				if len(fields) == 2 && fields[0] == languageName {
					entry := &ignoreEntry{value: fields[1], path: rel}
					inherited = append(inherited, entry)
					l.ignores = append(l.ignores, entry)
				} else if len(fields) == 3 && fields[0] == languageName && fields[1] == languageName {
					entry := &ignoreEntry{value: fields[2], path: rel}
					inherited = append(inherited, entry)
					l.ignores = append(l.ignores, entry)
				} else if len(fields) > 0 && fields[0] == languageName {
					l.fail("rust: //%s: malformed # gazelle:dx_ignore_import %s", rel, directive.Value)
				}
			}
		}
	}
	c.Exts[languageName] = &rustConfig{ignores: inherited, tools: tools}
}

func (l *rustLang) fail(format string, args ...interface{}) {
	l.errors = append(l.errors, fmt.Sprintf(format, args...))
}

func (l *rustLang) AfterResolvingDeps(context.Context) {
	for _, ignore := range l.ignores {
		if !ignore.used {
			l.fail("rust: //%s: stale # gazelle:dx_ignore_import rust %s matches no literal reference", ignore.path, ignore.value)
		}
	}
	if len(l.errors) == 0 {
		if l.manifest != nil {
			l.manifest.emit(l.ignores)
		}
		return
	}
	sort.Strings(l.errors)
	panic("Rust generation failed before BUILD emission:\n" + strings.Join(l.errors, "\n"))
}

func (*rustLang) Name() string { return languageName }

func (*rustLang) Kinds() map[string]rule.KindInfo { return rustKinds }

func (*rustLang) Loads() []rule.LoadInfo {
	return rustLoads("rules_dx", "crates", "rules_rust")
}

func (l *rustLang) ApparentLoads(moduleToApparentName func(string) string) []rule.LoadInfo {
	repoName := moduleToApparentName("rules_dx")
	if repoName == "" {
		repoName = "rules_dx"
	}
	cratesName := moduleToApparentName("crates")
	if cratesName == "" {
		cratesName = "crates"
	}
	rulesRustName := moduleToApparentName("rules_rust")
	if rulesRustName == "" {
		rulesRustName = "rules_rust"
	}
	return rustLoads(repoName, cratesName, rulesRustName)
}

func rustLoads(rulesRepo, cratesRepo, rulesRustRepo string) []rule.LoadInfo {
	return []rule.LoadInfo{
		{Name: "@" + rulesRepo + "//rust/rules:defs.bzl", Symbols: []string{binaryKind, libraryKind, testKind, procMacroKind, sharedKind, staticKind}},
		{Name: "@" + rulesRustRepo + "//cargo:defs.bzl", Symbols: []string{scriptKind}},
		{Name: "@" + cratesRepo + "//:crates.bzl", Symbols: []string{"aliases", "crate_deps"}},
		nativeConfigLoads(rulesRepo),
	}
}

func (*rustLang) Imports(_ *config.Config, r *rule.Rule, _ *rule.File) []resolve.ImportSpec {
	// Libraries and procedural-macro libraries both export linkable
	// crates: first-party path dependencies resolve to either. Shared and
	// static libraries cannot be depended on, and binaries (including
	// emitted examples and benches) are never cross-package providers.
	if r.Kind() != libraryKind && r.Kind() != procMacroKind {
		return nil
	}
	crateName := r.AttrString("crate_name")
	if crateName == "" {
		crateName = r.Name()
	}
	return []resolve.ImportSpec{{Lang: languageName, Imp: crateName}}
}

func (*rustLang) Embeds(*rule.Rule, label.Label) []label.Label { return nil }

func (l *rustLang) GenerateRules(args language.GenerateArgs) language.GenerateResult {
	res := l.generateRules(args)
	if l.manifest != nil {
		l.manifest.record(args, res)
	}
	return res
}

func (l *rustLang) generateRules(args language.GenerateArgs) language.GenerateResult {
	plan, err := planNativeConfig(args.Config, args)
	if err != nil {
		l.fail("%v", err)
		return language.GenerateResult{}
	}
	files, err := sourceFiles(args.Dir, args.Rel)
	if err != nil {
		l.fail("rust: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}
	for _, name := range args.RegularFiles {
		if name == "Cargo.toml" {
			return l.generateCargo(args, files, plan)
		}
	}
	if len(files) == 0 {
		return l.attachNative(args, language.GenerateResult{}, plan)
	}

	roots := DiscoverCrateRoots(args.Rel, files)
	if !roots.HasRoots() {
		return l.attachNative(args, language.GenerateResult{}, plan)
	}
	read := func(name string) ([]byte, error) {
		return os.ReadFile(filepath.Join(args.Config.RepoRoot, filepath.FromSlash(name)))
	}
	existsSet := make(map[string]bool, len(files))
	for _, name := range files {
		existsSet[name] = true
	}
	exists := func(name string) bool { return existsSet[name] }

	trees := make(map[string]map[string]*FileFacts)
	owners := make(map[string]string)
	for _, root := range append(append([]string{}, roots.LibRoot, roots.BinRoot), roots.TestRoots...) {
		if root == "" {
			continue
		}
		tree, loadErr := LoadCrate(root, read, exists)
		if loadErr != nil {
			l.fail("%v", loadErr)
			return language.GenerateResult{}
		}
		trees[root] = tree
		if err := claimSources(owners, root, tree); err != nil {
			l.fail("%v", err)
			return language.GenerateResult{}
		}
	}
	shape, err := ShapeCrate(roots, hasUnitTests(trees[roots.LibRoot]), hasUnitTests(trees[roots.BinRoot]))
	if err != nil {
		l.fail("%v", err)
		return language.GenerateResult{}
	}

	var result language.GenerateResult
	if shape.LibTarget != "" {
		r := crateRule(libraryKind, shape.LibTarget, shape.Name, roots.LibRoot, trees[roots.LibRoot], args.Rel)
		result.Gen = append(result.Gen, r)
		result.Imports = append(result.Imports, importsFor(trees[roots.LibRoot]))
		if shape.LibUnitTest {
			t := rule.NewRule(testKind, UnitTestName(shape.LibTarget))
			t.SetAttr("crate", ":"+shape.LibTarget)
			result.Gen = append(result.Gen, t)
			result.Imports = append(result.Imports, targetImports{test: importsFor(trees[roots.LibRoot]).test})
		}
	}
	if shape.BinTarget != "" {
		r := crateRule(binaryKind, shape.BinTarget, shape.Name, roots.BinRoot, trees[roots.BinRoot], args.Rel)
		result.Gen = append(result.Gen, r)
		result.Imports = append(result.Imports, importsFor(trees[roots.BinRoot]))
		if shape.BinUnitTest {
			t := rule.NewRule(testKind, UnitTestName(shape.BinTarget))
			t.SetAttr("crate", ":"+shape.BinTarget)
			result.Gen = append(result.Gen, t)
			result.Imports = append(result.Imports, targetImports{test: importsFor(trees[roots.BinRoot]).test})
		}
	}
	for _, test := range shape.Tests {
		r := crateRule(testKind, test.Name, strings.TrimSuffix(path.Base(test.Root), ".rs"), test.Root, trees[test.Root], args.Rel)
		result.Gen = append(result.Gen, r)
		result.Imports = append(result.Imports, importsFor(trees[test.Root]))
	}
	return l.attachNative(args, result, plan)
}

// attachNative folds the native-config plan into a generation result:
// planned config rules join the generated set before claim validation,
// Rust rules bind their aspect_hints, and planned removals join the
// generic stale sweep. Claim collisions stay fail-closed with no partial
// result.
func (l *rustLang) attachNative(args language.GenerateArgs, result language.GenerateResult, plan *nativePlan) language.GenerateResult {
	for _, r := range result.Gen {
		applyNativeHints(args.Rel, r, plan)
	}
	result.Gen = append(result.Gen, plan.gen...)
	result.Imports = append(result.Imports, plan.imports...)
	if err := checkExistingClaims(args.File, args.OtherGen, result.Gen); err != nil {
		l.fail("%v", err)
		return language.GenerateResult{}
	}
	merged := mergeStale(args.File, result)
	merged.Empty = append(merged.Empty, plan.empty...)
	return merged
}

func (l *rustLang) generateCargo(args language.GenerateArgs, files []string, plan *nativePlan) language.GenerateResult {
	manifestPath := path.Join(args.Rel, "Cargo.toml")
	content, err := os.ReadFile(filepath.Join(args.Dir, "Cargo.toml"))
	if err != nil {
		l.fail("rust: %s: %v", manifestPath, err)
		return language.GenerateResult{}
	}
	manifest, err := parseCargoManifest(manifestPath, content)
	if err != nil {
		l.fail("%v", err)
		return language.GenerateResult{}
	}
	existsSet := make(map[string]bool, len(files))
	for _, name := range files {
		existsSet[name] = true
	}
	if err := manifest.withImplicitTargets(existsSet, args.Rel); err != nil {
		l.fail("rust: %s: %v", manifestPath, err)
		return language.GenerateResult{}
	}
	if err := checkPathDepVersions(args.Dir, manifestPath, manifest); err != nil {
		l.fail("rust: %v", err)
		return language.GenerateResult{}
	}
	read := func(name string) ([]byte, error) {
		return os.ReadFile(filepath.Join(args.Config.RepoRoot, filepath.FromSlash(name)))
	}
	var result language.GenerateResult
	owners := make(map[string]string)
	for _, target := range manifest.targets {
		root := path.Join(args.Rel, target.path)
		if !existsSet[root] {
			if (target.kind == libraryKind && target.path == "src/lib.rs") || (target.kind == binaryKind && target.path == "src/main.rs") {
				continue
			}
			l.fail("rust: %s: Cargo target %s(%s) root %s does not exist", manifestPath, target.kind, target.name, target.path)
			continue
		}
		tree, loadErr := LoadCrate(root, read, func(name string) bool { return existsSet[name] })
		if loadErr != nil {
			l.fail("%v", loadErr)
			continue
		}
		if err := claimSources(owners, target.name, tree); err != nil {
			l.fail("%v", err)
			continue
		}
		r := crateRule(dxKindFor(target), target.name, target.crate(), root, tree, args.Rel)
		r.SetAttr("edition", manifest.edition)
		imports := importsFor(tree)
		// Examples and benches link development dependencies: their
		// production imports may come from [dev-dependencies]. The
		// build script sees only [build-dependencies]; it is emitted
		// separately below.
		if target.kind == exampleKind || target.kind == benchKind {
			if err := validateExampleImports(args.Config, manifest, imports); err != nil {
				l.fail("rust: %s: target %s: %v", manifestPath, target.name, err)
				continue
			}
		} else if err := validateCargoImports(args.Config, manifest, target.kind, imports); err != nil {
			l.fail("rust: %s: target %s: %v", manifestPath, target.name, err)
			continue
		}
		includeDev := target.kind == testKind || target.kind == exampleKind || target.kind == benchKind
		setCargoAttrs(r, args.Rel, manifest, imports, includeDev)
		var resultImports targetImports
		if target.kind == exampleKind || target.kind == benchKind {
			resultImports = localCargoExampleImports(args.Config, manifest, imports)
		} else {
			resultImports = localCargoImports(args.Config, manifest, imports, includeDev)
		}
		if dxKindFor(target) == binaryKind {
			resultImports.siblingLib = siblingLibName(manifest, target)
		}
		if target.kind == testKind && !target.harness {
			r.SetAttr("use_libtest_harness", false)
		}
		result.Gen = append(result.Gen, r)
		result.Imports = append(result.Imports, resultImports)
		if wantsUnitTest(target, tree) {
			// The unit-test wrapper is named after the Rust crate, not the
			// Bazel target: `crate` keeps pointing at the (possibly
			// disambiguated) library target while the test name stays
			// stable across lib renames. A binary with its own unit tests
			// takes the `_bin` variant so lib and bin wrappers never share
			// a name in one package; an example with `test = true` takes
			// the `<example>_test` wrapper. Benches never gain wrappers.
			var testName string
			switch target.kind {
			case exampleKind:
				testName = ExampleTestName(target.name)
			case binaryKind:
				testName = UnitTestName(target.name + "_bin")
			default:
				testName = UnitTestName(target.crate())
			}
			t := rule.NewRule(testKind, testName)
			t.SetAttr("crate", ":"+target.name)
			setCargoAttrs(t, args.Rel, manifest, targetImports{test: imports.test}, true)
			result.Gen = append(result.Gen, t)
			result.Imports = append(result.Imports, localCargoImports(args.Config, manifest, targetImports{test: imports.test}, true))
		}
	}
	if manifest.build != nil && !manifest.build.disabled {
		l.emitBuildScript(args, manifestPath, manifest, existsSet, read, owners, &result)
	}
	if len(l.errors) > 0 {
		return language.GenerateResult{}
	}
	return l.attachNative(args, result, plan)
}

// dxKindFor maps a manifest target to its emitted rule kind: examples and
// benches are ordinary binaries under affixed names; libraries resolve
// their flavor to the matching wrapper.
func dxKindFor(target cargoTarget) string {
	switch target.kind {
	case exampleKind, benchKind:
		return binaryKind
	case libraryKind:
		switch target.flavor {
		case "proc-macro":
			return procMacroKind
		case "cdylib":
			return sharedKind
		case "staticlib":
			return staticKind
		}
	}
	return target.kind
}

// wantsUnitTest reports whether a target gains a libtest wrapper: every
// crate kind except integration tests (which already are tests) and
// benches (which never run under libtest); examples only with
// `test = true`.
func wantsUnitTest(target cargoTarget, tree map[string]*FileFacts) bool {
	if target.kind == testKind || target.kind == benchKind {
		return false
	}
	if target.kind == exampleKind && !target.exampleTest {
		return false
	}
	return hasUnitTests(tree)
}

// emitBuildScript generates the cargo_build_script rule for an active
// [package] build script and links every crate rule already in result to
// it: the upstream consumer pattern is a plain deps edge carrying
// BuildScriptInfo outputs (cfgs, env, generated files) into each crate's
// compilation. The script rule itself stays unlinked. Generated script
// attributes mirror crate_universe's script shape (srcs, crate_root,
// edition, version, pkg_name, crate_features) with hermetic tristates
// forced (use_cc_toolchain on, default shell env off) and diagnostics
// forwarded (emit_warnings on, overridable by the global build setting).
// tools, data, env, and links stay user-owned via keep: a script needing
// them fails in the sandbox rather than building silently wrong.
func (l *rustLang) emitBuildScript(args language.GenerateArgs, manifestPath string, manifest *cargoManifest, existsSet map[string]bool, read func(string) ([]byte, error), owners map[string]string, result *language.GenerateResult) {
	scriptName := BuildScriptName(manifest.packageName)
	root := path.Join(args.Rel, manifest.build.path)
	if !existsSet[root] {
		l.fail("rust: %s: build script %s does not exist", manifestPath, manifest.build.path)
		return
	}
	tree, loadErr := LoadCrate(root, read, func(name string) bool { return existsSet[name] })
	if loadErr != nil {
		l.fail("%v", loadErr)
		return
	}
	if err := claimSources(owners, scriptName, tree); err != nil {
		l.fail("%v", err)
		return
	}
	imports := importsFor(tree)
	if err := validateBuildImports(args.Config, manifest, imports); err != nil {
		l.fail("rust: %s: build script: %v", manifestPath, err)
		return
	}
	r := rule.NewRule(scriptKind, scriptName)
	rel := manifest.build.path
	r.SetAttr("srcs", []string{rel})
	r.SetAttr("crate_root", rel)
	r.SetAttr("crate_name", strings.ReplaceAll(scriptName, "-", "_"))
	r.SetAttr("edition", manifest.edition)
	if manifest.version != "" {
		r.SetAttr("version", manifest.version)
	}
	r.SetAttr("pkg_name", manifest.packageName)
	r.SetAttr("crate_features", []string{})
	r.SetAttr("emit_warnings", true)
	r.SetAttr("use_cc_toolchain", 1)
	r.SetAttr("use_default_shell_env", 0)
	setScriptAttrs(r, args.Rel, manifest)
	for i, raw := range result.Imports {
		if current, ok := raw.(targetImports); ok {
			current.scriptDep = scriptName
			result.Imports[i] = current
		}
	}
	result.Gen = append(result.Gen, r)
	result.Imports = append(result.Imports, localCargoBuildImports(args.Config, manifest, imports))
}

func claimSources(owners map[string]string, owner string, tree map[string]*FileFacts) error {
	for source := range tree {
		if previous, ok := owners[source]; ok {
			return fmt.Errorf("rust: source %s is owned by both %s and %s", source, previous, owner)
		}
		owners[source] = owner
	}
	return nil
}

func checkExistingClaims(file *rule.File, other, generated []*rule.Rule) error {
	claims := make(map[string]string)
	if file != nil {
		for _, existing := range file.Rules {
			claims[existing.Name()] = existing.Kind()
		}
	}
	for _, existing := range other {
		claims[existing.Name()] = existing.Kind()
	}
	for _, proposed := range generated {
		if kind, ok := claims[proposed.Name()]; ok && kind != proposed.Kind() {
			return fmt.Errorf("rust: target name %q is claimed by generated %s and existing %s", proposed.Name(), proposed.Kind(), kind)
		}
		claims[proposed.Name()] = proposed.Kind()
	}
	return nil
}

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
	return result
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
	return result
}

func setCargoAttrs(r *rule.Rule, packagePath string, manifest *cargoManifest, imports targetImports, includeDev bool) {
	// crate_universe keys its maps by parent dir + Cargo package name
	// (e.g. dx/dx_output for //dx/output), not by Bazel package path.
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

type crateDepsCall struct {
	names       []string
	packageName string
}

func (c crateDepsCall) BzlExpr() bzl.Expr {
	return &bzl.CallExpr{X: &bzl.Ident{Name: "crate_deps"}, List: []bzl.Expr{
		rule.ExprFromValue(c.names),
		&bzl.AssignExpr{LHS: &bzl.Ident{Name: "package_name"}, Op: "=", RHS: &bzl.StringExpr{Value: c.packageName}},
	}}
}

func (c crateDepsCall) Merge(other bzl.Expr) bzl.Expr {
	managed := make(map[string]bool, len(c.names))
	for _, name := range c.names {
		managed[name] = true
	}
	if hand := unmanagedDepsLabels(other, managed); len(hand) > 0 {
		return depsConcatExpr{base: c.BzlExpr(), extra: hand}.BzlExpr()
	}
	return c.BzlExpr()
}

// depsConcatExpr renders `base + [...]`: first-party labels Resolve appends
// to generated crate_deps(...) calls. Both sides are label lists, so the
// concatenation stays a valid deps list. Merge takes the freshly resolved
// value but carries forward hand-maintained labels from the previous
// expression (e.g. `@rules_rust//tools/runfiles:runfiles`, which no import
// or directive resolves): dropping them would silently break the build on
// every generate. Hand labels are therefore never removed by generate;
// delete them manually when they go stale.
type depsConcatExpr struct {
	base  bzl.Expr
	extra []string
}

func (c depsConcatExpr) BzlExpr() bzl.Expr {
	extra := make([]bzl.Expr, len(c.extra))
	for i, dep := range c.extra {
		extra[i] = &bzl.StringExpr{Value: dep}
	}
	return &bzl.BinaryExpr{X: c.base, Op: "+", Y: &bzl.ListExpr{List: extra}}
}

func (c depsConcatExpr) Merge(other bzl.Expr) bzl.Expr {
	managed := make(map[string]bool, len(c.extra))
	for _, dep := range c.extra {
		managed[dep] = true
	}
	for _, name := range cargoCallNames(c.base) {
		managed[name] = true
	}
	return depsConcatExpr{base: c.base, extra: unionStrings(c.extra, unmanagedDepsLabels(other, managed))}.BzlExpr()
}

// cargoCallNames collects the crate names referenced by a crate_deps call
// expression so Merge can tell managed names apart from hand labels. The
// base is always a rendered *bzl.CallExpr (rule attrs store BzlExpr()
// output, never the wrapper struct).
func cargoCallNames(base bzl.Expr) []string {
	call, ok := base.(*bzl.CallExpr)
	if !ok {
		return nil
	}
	if ident, ok := call.X.(*bzl.Ident); !ok || ident.Name != "crate_deps" {
		return nil
	}
	if len(call.List) == 0 {
		return nil
	}
	list, ok := call.List[0].(*bzl.ListExpr)
	if !ok {
		return nil
	}
	var names []string
	for _, item := range list.List {
		if s, ok := item.(*bzl.StringExpr); ok {
			names = append(names, s.Value)
		}
	}
	return names
}

// unmanagedDepsLabels returns the plain-list string labels in a previous
// deps expression that are not in the managed set. Only ListExpr nodes
// (including `+` tails) contribute: strings inside call arguments such as
// crate_deps' package_name are metadata, not labels, and are skipped.
func unmanagedDepsLabels(other bzl.Expr, managed map[string]bool) []string {
	var out []string
	var walk func(e bzl.Expr)
	walk = func(e bzl.Expr) {
		switch e := e.(type) {
		case *bzl.ListExpr:
			for _, item := range e.List {
				if s, ok := item.(*bzl.StringExpr); ok && !managed[s.Value] {
					out = append(out, s.Value)
				}
			}
		case *bzl.BinaryExpr:
			if e.Op == "+" {
				walk(e.X)
				walk(e.Y)
			}
		}
	}
	if other != nil {
		walk(other)
	}
	sort.Strings(out)
	return out
}

type cargoCallExpr struct {
	name         string
	packageName  string
	includeDev   bool
	includeBuild bool
}

func cargoCall(name, packageName string, includeDev bool) cargoCallExpr {
	return cargoCallExpr{name: name, packageName: packageName, includeDev: includeDev}
}

func cargoBuildCall(packageName string) cargoCallExpr {
	return cargoCallExpr{name: "aliases", packageName: packageName, includeBuild: true}
}

func (c cargoCallExpr) BzlExpr() bzl.Expr {
	var args []bzl.Expr
	// Build scope selects exactly the build maps, mirroring
	// crate_universe's aliases(build = True); every other call keeps the
	// historical normal-first shape.
	if !c.includeBuild {
		args = append(args, &bzl.AssignExpr{LHS: &bzl.Ident{Name: "normal"}, Op: "=", RHS: &bzl.Ident{Name: "True"}})
	}
	args = append(args, &bzl.AssignExpr{LHS: &bzl.Ident{Name: "package_name"}, Op: "=", RHS: &bzl.StringExpr{Value: c.packageName}})
	if c.includeDev {
		args = append(args, &bzl.AssignExpr{LHS: &bzl.Ident{Name: "normal_dev"}, Op: "=", RHS: &bzl.Ident{Name: "True"}})
	}
	if c.includeBuild {
		args = append(args, &bzl.AssignExpr{LHS: &bzl.Ident{Name: "build"}, Op: "=", RHS: &bzl.Ident{Name: "True"}})
	}
	return &bzl.CallExpr{X: &bzl.Ident{Name: c.name}, List: args}
}

func (c cargoCallExpr) Merge(bzl.Expr) bzl.Expr { return c.BzlExpr() }

func sourceFiles(dir, rel string) ([]string, error) {
	var files []string
	for _, root := range []string{dir} {
		err := filepath.WalkDir(root, func(name string, entry os.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if entry.IsDir() && name != root {
				if strings.HasPrefix(entry.Name(), ".") || entry.Name() == "bazel-bin" || entry.Name() == "bazel-out" {
					return filepath.SkipDir
				}
				if _, statErr := os.Stat(filepath.Join(name, "BUILD.bazel")); statErr == nil {
					return filepath.SkipDir
				}
				if _, statErr := os.Stat(filepath.Join(name, "BUILD")); statErr == nil {
					return filepath.SkipDir
				}
			}
		if !entry.IsDir() && strings.HasSuffix(entry.Name(), ".rs") {
			local, relErr := filepath.Rel(dir, name)
			// LCOV_EXCL_START - reason: WalkDir only yields descendants of dir, so Rel cannot fail on Linux; this branch is defensive only.
			if relErr != nil {
				return relErr
			}
			// LCOV_EXCL_STOP - reason: end of unreachable Rel-failure guard.
			files = append(files, path.Join(rel, filepath.ToSlash(local)))
			}
			return nil
		})
		if err != nil {
			return nil, err
		}
	}
	sort.Strings(files)
	return files, nil
}

func hasUnitTests(tree map[string]*FileFacts) bool {
	for _, facts := range tree {
		if facts.HasTestAttr || facts.HasCfgTest || facts.InnerCfgTest {
			return true
		}
	}
	return false
}

func crateRule(kind, name, crateName, root string, tree map[string]*FileFacts, pkg string) *rule.Rule {
	r := rule.NewRule(kind, name)
	srcs := make([]string, 0, len(tree))
	for source := range tree {
		srcs = append(srcs, strings.TrimPrefix(source, strings.TrimSuffix(pkg, "/")+"/"))
	}
	sort.Strings(srcs)
	r.SetAttr("srcs", srcs)
	r.SetAttr("crate_root", strings.TrimPrefix(root, strings.TrimSuffix(pkg, "/")+"/"))
	r.SetAttr("crate_name", crateName)
	return r
}

func importsFor(tree map[string]*FileFacts) targetImports {
	sets := [2]map[string]bool{{}, {}}
	localModules := make(map[string]bool)
	for _, facts := range tree {
		for _, module := range facts.Modules {
			localModules[module.Name] = true
		}
	}
	for _, facts := range tree {
		for _, use := range facts.Uses {
			addImport(sets, localModules, use.Path, use.CfgTest)
		}
		for _, ext := range facts.Externs {
			// An `as` alias renames the crate for every later path in the
			// crate, so the alias is the name validation and resolution see;
			// an exact mapping then pins the alias to the real target.
			name := ext.Name
			if ext.As != "" {
				name = ext.As
			}
			addImport(sets, localModules, name, ext.CfgTest)
		}
	}
	result := targetImports{}
	for imp := range sets[0] {
		result.production = append(result.production, imp)
	}
	for imp := range sets[1] {
		result.test = append(result.test, imp)
	}
	sort.Strings(result.production)
	sort.Strings(result.test)
	return result
}

func addImport(sets [2]map[string]bool, localModules map[string]bool, raw string, test bool) {
	root := strings.TrimPrefix(strings.Split(raw, "::")[0], "::")
	if root == "" || localModules[root] || root == "crate" || root == "self" || root == "super" || root == "std" || root == "core" || root == "alloc" {
		return
	}
	index := 0
	if test {
		index = 1
	}
	sets[index][root] = true
}

func mergeStale(file *rule.File, result language.GenerateResult) language.GenerateResult {
	desired := make(map[string]bool, len(result.Gen))
	for _, r := range result.Gen {
		desired[r.Kind()+"\x00"+r.Name()] = true
	}
	stale := staleRules(file, desired)
	result.Empty = append(result.Empty, stale.Empty...)
	return result
}

func staleRules(file *rule.File, desired map[string]bool) language.GenerateResult {
	var result language.GenerateResult
	if file == nil {
		return result
	}
	for _, existing := range file.Rules {
		// Managed config targets never sweep here: a hand-authored
		// target of a config kind is always preserved, and only the
		// native plan stages exact removals of generated rules.
		if isNativeConfigKind(existing.Kind()) {
			continue
		}
		if _, owned := rustKinds[existing.Kind()]; !owned || desired[existing.Kind()+"\x00"+existing.Name()] {
			continue
		}
		empty := rule.NewRule(existing.Kind(), existing.Name())
		result.Empty = append(result.Empty, empty)
	}
	return result
}

// siblingLibName returns the Bazel name of the same-manifest library a
// binary target links automatically, or "" when the package has no library.
// An ordinary library wins; otherwise the first flavored library binds
// (proc-macro, cdylib, staticlib): Cargo links examples, benches, and bins
// against the package library whatever its shape, and a link failure
// upstream then means Cargo would fail too.
func siblingLibName(manifest *cargoManifest, bin cargoTarget) string {
	for _, target := range manifest.targets {
		if target.kind == libraryKind && target.flavor == "" {
			return target.name
		}
	}
	for _, target := range manifest.targets {
		if target.kind == libraryKind {
			return target.name
		}
	}
	return ""
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
	if imports.siblingLib != "" && imports.siblingLib != from.Name {
		deps[":"+imports.siblingLib] = true
	}
	if imports.scriptDep != "" && imports.scriptDep != from.Name {
		deps[":"+imports.scriptDep] = true
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
