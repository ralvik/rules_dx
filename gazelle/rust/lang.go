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
	"github.com/bazelbuild/bazel-gazelle/resolve"
)

const (
	languageName = "rust"
	libraryKind  = "rust_library"
	binaryKind   = "rust_binary"
	testKind     = "rust_test"
	// procMacroKind, sharedKind, and staticKind are the dx wrappers for
	// Cargo [lib] targets with proc-macro = true, crate-type =
	// ["cdylib"], and crate-type = ["staticlib"].
	procMacroKind = "rust_proc_macro"
	sharedKind    = "rust_shared_library"
	staticKind    = "rust_static_library"
	// scriptKind is the upstream cargo_build_script wrapper macro (loaded
	// from @rules_rust//cargo), not a dx wrapper: the macro already owns
	// the script-binary/runfiles split and stays self-describing.
	scriptKind = "cargo_build_script"
	// dxCrateKind is the leaf-crate boilerplate macro: it
	// expands to `<name>` rust_library + `<name>_test` rust_test + lint
	// tests + manifest export. Corpus splits are owned by generation
	// , never by this macro. BUILD files hand-maintain it;
	// generation must recognize it as covering the lib + unit-test it
	// emits instead of proposing duplicate rust_library/rust_test rules.
	dxCrateKind = "dx_rust_crate"
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

// isLibraryKind reports whether a rule kind is a linkable library flavor:
// only libraries are ever valid cross-package dependencies in rules_rust.
func isLibraryKind(kind string) bool {
	switch kind {
	case libraryKind, procMacroKind, sharedKind, staticKind:
		return true
	}
	return false
}

// shouldSetVisibility reports whether generated library rules need an
// explicit public visibility: a Cargo workspace links every crate from
// every other crate, while Bazel defaults to private. Files that already
// declare a default visibility keep it (their owner opted out
// explicitly). Bins, tests, and scripts never gain visibility: they are
// never valid cross-package dependencies, and same-package references
// (sibling lib, unit-test crate edge, build script) work under the
// private default.
func shouldSetVisibility(args language.GenerateArgs) bool {
	if args.File != nil && args.File.HasDefaultVisibility() {
		return false
	}
	for _, r := range args.OtherGen {
		if r.Kind() == "package" && r.Attr("default_visibility") != nil {
			return false
		}
	}
	return true
}

func init() {
	// Managed native-config targets merge through the same file the
	// Rust rules live in, so their kinds register alongside. Corpus
	// splits (`real_source_target` per content type,) merge
	// through the same file as well: every corpus block is written and
	// maintained by this workflow (`dx generate`).
	for kind, info := range nativeConfigKinds() {
		rustKinds[kind] = info
	}
	for kind, info := range corpusKinds() {
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
	// mirrorPaths links every declared first-party path dependency from
	// the target's visible scopes whether or not any use item names it:
	// Cargo links all declared dependencies into a target, while import
	// detection only sees use items (an expression path like
	// `api::digest(words)` never surfaces). Resolution looks these names
	// up in the rule index but stays silent on misses and ambiguities:
	// a declared-but-unused dep is legal, and a used-but-undetected one
	// that the index cannot place unambiguously is reported by rustc,
	// not by fail-closed generation. Validation ignores this set.
	mirrorPaths []string
	// siblingLib is the Bazel name of the same-package library a binary,
	// test, example, or bench target links automatically (Cargo binds
	// the sibling lib without an import). Empty for libraries and
	// scripts. It flows straight to deps: validation and manifest
	// indexing ignore it.
	siblingLib string
	// scriptDep is the Bazel name of the package's generated build-script
	// rule. Every crate rule in a package with an active script links it
	// (the upstream consumer pattern is a plain deps edge), except the
	// script rule itself. It flows straight to deps like siblingLib.
	scriptDep string
}

// NewLanguage returns the private first-party Rust Gazelle extension.
func NewLanguage() language.Language { return &rustLang{} }

// exitProcess ends the Gazelle run when generation errors are recorded.
// It is a variable so unit tests can observe the fail-closed decision
// without exiting the test process.
var exitProcess = os.Exit

func (l *rustLang) Before(context.Context) {
	l.errors = nil
	l.ignores = nil
	l.manifest = nil
	if rec, err := loadManifestRecorder(); err != nil {
		l.fail("%v", err)
	} else {
		l.manifest = rec
	}
	if l.manifest != nil {
		l.manifest.apparentLoads = l.ApparentLoads
	}
}

func (*rustLang) DoneGeneratingRules() {}

func (l *rustLang) RegisterFlags(*flag.FlagSet, string, *config.Config) {}

func (l *rustLang) CheckFlags(*flag.FlagSet, *config.Config) error { return nil }

func (*rustLang) KnownDirectives() []string {
	return []string{"dx_ignore_import", nativeToolsDirective}
}

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
	if len(l.errors) == 0 && l.manifest != nil {
		if err := l.manifest.emit(l.ignores); err != nil {
			l.fail("%v", err)
		}
	}
	// The framework's Language interface offers no error return here, so a
	// fatal exit — not a panic and its stack trace — is the only way to
	// fail the run before BUILD emission. A zero-length error list returns
	// normally.
	if len(l.errors) > 0 {
		sort.Strings(l.errors)
		fmt.Fprintln(os.Stderr, "Rust generation failed before BUILD emission:\n"+strings.Join(l.errors, "\n"))
		exitProcess(1)
	}
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
		{Name: "@" + rulesRepo + "//rust/rules:defs.bzl", Symbols: []string{binaryKind, libraryKind, testKind, procMacroKind, sharedKind, staticKind, dxCrateKind}},
		{Name: "@" + rulesRustRepo + "//cargo:defs.bzl", Symbols: []string{scriptKind}},
		{Name: "@" + cratesRepo + "//:crates.bzl", Symbols: []string{"aliases", "crate_deps"}},
		nativeConfigLoads(rulesRepo),
		corpusLoads(rulesRepo),
	}
}

func (*rustLang) Imports(_ *config.Config, r *rule.Rule, _ *rule.File) []resolve.ImportSpec {
	// Libraries and procedural-macro libraries both export linkable
	// crates: first-party path dependencies resolve to either. Shared and
	// static libraries cannot be depended on, and binaries (including
	// emitted examples and benches) are never cross-package providers.
	// The dx_rust_crate macro expands to an ordinary rust_library, so it
	// provides the same import as the library it emits (crate_name attr
	// or the macro name when omitted).
	if r.Kind() == dxCrateKind {
		crateName := r.AttrString("crate_name")
		if crateName == "" {
			crateName = r.Name()
		}
		return []resolve.ImportSpec{{Lang: languageName, Imp: crateName}}
	}
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
	publicLibs := shouldSetVisibility(args)
	if shape.LibTarget != "" {
		r := crateRule(libraryKind, shape.LibTarget, shape.Name, roots.LibRoot, trees[roots.LibRoot], args.Rel)
		if publicLibs {
			r.SetAttr("visibility", []string{"//visibility:public"})
		}
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

// attachNative folds the native-config plan and the corpus split plan
// into a generation result: planned config rules join the
// generated set before claim validation, Rust rules bind their
// aspect_hints, corpus splits join as fully owned targets, and planned
// removals join the generic stale sweep. Claim collisions stay
// fail-closed with no partial result. Rules already covered by a
// hand-maintained dx_rust_crate macro (leaf-crate lib + unit-test) are
// filtered before validation so the macro stays the single owner and no
// duplicate target is proposed. Corpus splits are never filtered: the
// macro no longer emits `corpus` (Gazelle owns every corpus block), so a
// macro package still gains its `corpus_*` targets from generation.
func (l *rustLang) attachNative(args language.GenerateArgs, result language.GenerateResult, plan *nativePlan) language.GenerateResult {
	result = filterDxCrateCovered(args.File, result)
	for _, r := range result.Gen {
		applyNativeHints(args.Rel, r, plan)
	}
	result.Gen = append(result.Gen, plan.gen...)
	result.Imports = append(result.Imports, plan.imports...)
	hasOtherGen := len(result.Gen) > 0 || args.File != nil
	corpus := planCorpus(args, hasOtherGen)
	result.Gen = append(result.Gen, corpus.gen...)
	// Corpus imports parallel Gen with empty targetImports so Resolve
	// stays aligned; corpus rules never resolve deps.
	for range corpus.gen {
		result.Imports = append(result.Imports, targetImports{})
	}
	// Auto-testonly for fixture paths: generated rules under
	// tests/fixtures/testdata carry testonly.
	if isFixturePath(args.Rel) {
		for _, r := range result.Gen {
			r.SetAttr("testonly", true)
		}
	}
	if err := checkExistingClaims(args.File, args.OtherGen, result.Gen); err != nil {
		l.fail("%v", err)
		return language.GenerateResult{}
	}
	merged := mergeStale(args.File, result)
	merged.Empty = append(merged.Empty, plan.empty...)
	merged.Empty = append(merged.Empty, corpus.empty...)
	return merged
}

// dxCrateNames collects the macro names of hand-maintained dx_rust_crate
// rules in a BUILD file. Nil files yield no names.
func dxCrateNames(file *rule.File) map[string]bool {
	names := make(map[string]bool)
	if file == nil {
		return names
	}
	for _, existing := range file.Rules {
		if existing.Kind() == dxCrateKind {
			names[existing.Name()] = true
		}
	}
	return names
}

// filterDxCrateCovered drops generated rules already provided by a
// hand-maintained dx_rust_crate macro in the same package: the ordinary
// rust_library with the macro name and the unit-test wrapper
// `<name>_test` via `crate = ":<name>"`. Integration tests (no crate
// edge), binaries, build scripts, and flavored libraries (proc-macro,
// cdylib, staticlib) are never covered: the macro only emits the
// lib-only leaf pattern, so those stay generated and any true conflict
// still fails closed in checkExistingClaims.
func filterDxCrateCovered(file *rule.File, result language.GenerateResult) language.GenerateResult {
	covered := dxCrateNames(file)
	if len(covered) == 0 || len(result.Gen) == 0 {
		return result
	}
	keptGen := result.Gen[:0]
	keptImports := result.Imports[:0]
	for i, r := range result.Gen {
		if r.Kind() == libraryKind && covered[r.Name()] {
			continue
		}
		if r.Kind() == testKind && r.Attr("crate") != nil {
			if crate := r.AttrString("crate"); len(crate) > 1 && crate[0] == ':' {
				if covered[crate[1:]] && r.Name() == crate[1:]+"_test" {
					continue
				}
			}
		}
		keptGen = append(keptGen, r)
		if i < len(result.Imports) {
			keptImports = append(keptImports, result.Imports[i])
		}
	}
	// When Gen is empty but Imports held only plan-independent entries,
	// keep the slices consistent for the caller.
	result.Gen = keptGen
	if len(result.Gen) == 0 {
		// Preserve any trailing imports only when they still align;
		// filtered lib/test imports drop with their rules.
		if len(keptImports) > len(keptGen) {
			keptImports = keptImports[:len(keptGen)]
		}
		result.Imports = keptImports
	} else {
		// Imports parallel Gen for crate rules; truncation above already
		// keeps alignment when every Gen entry had an import.
		if len(result.Imports) != len(keptImports) {
			result.Imports = keptImports
		}
	}
	return result
}

// CollectUsedIgnores reports used dx_ignore_import entries visible in c
// as (path, value) pairs for the composed `//dx:generate` witness.
// See: docs/cli/commands/generate.md.
func CollectUsedIgnores(c *config.Config) [][2]string {
	raw, ok := c.Exts[languageName]
	if !ok || raw == nil {
		return nil
	}
	cfg, ok := raw.(*rustConfig)
	if !ok || cfg == nil {
		return nil
	}
	seen := map[[2]string]bool{}
	var out [][2]string
	for _, ig := range cfg.ignores {
		if ig == nil || !ig.used {
			continue
		}
		key := [2]string{ig.path, ig.value}
		if seen[key] {
			continue
		}
		seen[key] = true
		out = append(out, key)
	}
	return out
}
