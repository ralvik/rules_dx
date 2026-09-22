// Generation and rendering for the dx Rust Gazelle extension.

package rust

import (
	"fmt"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"

	"github.com/bazel-contrib/bazel-gazelle/v2/rule"
	"github.com/bazelbuild/bazel-gazelle/language"
	bzl "github.com/bazelbuild/buildtools/build"
)

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
	if manifest.virtual {
		return l.attachNative(args, language.GenerateResult{}, plan)
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
	publicLibs := shouldSetVisibility(args)
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
		if isLibraryKind(r.Kind()) && publicLibs {
			r.SetAttr("visibility", []string{"//visibility:public"})
		}
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
		// Tests link the sibling library like binaries do: Cargo binds
		// the package lib into integration tests without an import.
		// (Examples and benches already arrive here as binaryKind via
		// dxKindFor.) Unit-test wrappers below keep narrow test-only
		// imports instead: their `crate` edge carries the package.
		if dxKindFor(target) == binaryKind || target.kind == testKind {
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
			wrapperImports := localCargoImports(args.Config, manifest, targetImports{test: imports.test}, true)
			// The wrapper tests its own crate through the `crate`
			// edge: mirroring the package's declared path deps here
			// would only duplicate what the crate target already
			// links, so the wrapper stays narrow.
			wrapperImports.mirrorPaths = nil
			result.Imports = append(result.Imports, wrapperImports)
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
// edition, version, pkg_name, crate_features) with hermetic defaults
// forced (use_cc_toolchain on, default shell env off) and diagnostics
// forwarded (emit_warnings on, overridable by the global build setting).
// tools, data, env, and links stay user-owned via keep: a script needing
// them fails in the sandbox rather than building silently wrong.
func (l *rustLang) emitBuildScript(args language.GenerateArgs, manifestPath string, manifest *cargoManifest, existsSet map[string]bool, read func(string) ([]byte, error), owners map[string]string, result *language.GenerateResult) {
	scriptName, err := BuildScriptName(manifest.packageName)
	if err != nil {
		l.fail("rust: %s: %v", manifestPath, err)
		return
	}
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
			// A hand-maintained dx_rust_crate macro owns the ordinary
			// rust_library it expands to: the macro stays
			// the single owner and generation filters the covered lib
			// before this check, so an unfiltered residue must not fail
			// the run. Flavored libraries (proc-macro/cdylib/staticlib)
			// never match: the macro only emits ordinary rlibs.
			if kind == dxCrateKind && proposed.Kind() == libraryKind {
				continue
			}
			return fmt.Errorf("rust: target name %q is claimed by generated %s and existing %s", proposed.Name(), proposed.Kind(), kind)
		}
		claims[proposed.Name()] = proposed.Kind()
	}
	return nil
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
				// LCOV_EXCL_START - policy: docs/testing/README.md#coverage
				if relErr != nil {
					return relErr
				}
				// LCOV_EXCL_STOP - policy: docs/testing/README.md#coverage
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

// isFixturePath reports whether a Gazelle relative directory is a test-only
// fixture path: any path containing tests, fixtures, or
// testdata as a segment generates testonly targets.
func isFixturePath(rel string) bool {
	padded := "/" + rel + "/"
	return strings.Contains(padded, "/tests/") || strings.Contains(padded, "/fixtures/") || strings.Contains(padded, "/testdata/")
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
