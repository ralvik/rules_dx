package golang

import (
	"context"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/bazel-contrib/bazel-gazelle/v2/label"
	"github.com/bazel-contrib/bazel-gazelle/v2/rule"
	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/repo"
	"github.com/bazelbuild/bazel-gazelle/resolve"
)

const languageName = "go"

var goKinds = map[string]rule.KindInfo{
	LibraryKind: rule.KindInfo{
		MatchAttrs:    []string{"srcs"},
		NonEmptyAttrs: map[string]bool{"srcs": true},
		MergeableAttrs: map[string]bool{
			"srcs":       true,
			"deps":       true,
			"importpath": true,
		},
		ResolveAttrs: map[string]bool{"deps": true},
	},
	TestKind: rule.KindInfo{
		MatchAttrs:    []string{"srcs"},
		NonEmptyAttrs: map[string]bool{"srcs": true},
		MergeableAttrs: map[string]bool{
			"srcs":  true,
			"deps":  true,
			"embed": true,
		},
		ResolveAttrs: map[string]bool{"deps": true},
	},
}

type goLang struct {
	language.BaseLang
	errors  []string
	ignores []*ignoreEntry
}

type goConfig struct {
	ignores []*ignoreEntry
}

type ignoreEntry struct {
	value string
	path  string
	used  bool
}

// targetImports is the deduplicated union of normalized import roots for
// one generated library's non-test sources. Standard-library roots are
// dropped at collection; every other root resolves strictly or fails
// generation.
type targetImports struct {
	imports []string
}

// testTargetImports is the deduplicated union of normalized import roots for
// one generated package-level test's `*_test.go` sources. It resolves
// strictly like library imports; the owning library reaches the test via
// `embed`, never via an import edge.
type testTargetImports struct {
	imports []string
}

// NewLanguage returns the private first-party Go Gazelle extension.
func NewLanguage() language.Language { return &goLang{} }

func (l *goLang) Before(context.Context) { l.errors = nil; l.ignores = nil }

func (l *goLang) DoneGeneratingRules() {}

func (l *goLang) RegisterFlags(*flag.FlagSet, string, *config.Config) {}

func (l *goLang) CheckFlags(*flag.FlagSet, *config.Config) error { return nil }

func (*goLang) KnownDirectives() []string { return []string{"dx_ignore_import"} }

func (l *goLang) Configure(c *config.Config, rel string, f *rule.File) {
	var inherited []*ignoreEntry
	if raw, ok := c.Exts[languageName]; ok {
		inherited = append(inherited, raw.(*goConfig).ignores...)
	}
	if f != nil {
		for _, directive := range f.Directives {
			if directive.Key != "dx_ignore_import" {
				continue
			}
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
				l.fail("go: //%s: malformed # gazelle:dx_ignore_import %s", rel, directive.Value)
			}
		}
	}
	c.Exts[languageName] = &goConfig{ignores: inherited}
}

func (l *goLang) fail(format string, args ...interface{}) {
	l.errors = append(l.errors, fmt.Sprintf(format, args...))
}

func (l *goLang) AfterResolvingDeps(context.Context) {
	for _, ignore := range l.ignores {
		if !ignore.used {
			l.fail("go: //%s: stale # gazelle:dx_ignore_import go %s matches no literal reference", ignore.path, ignore.value)
		}
	}
	if len(l.errors) == 0 {
		return
	}
	sort.Strings(l.errors)
	panic("Go generation failed:\n" + strings.Join(l.errors, "\n"))
}

func (*goLang) Name() string { return languageName }

func (*goLang) Kinds() map[string]rule.KindInfo { return goKinds }

func (*goLang) Loads() []rule.LoadInfo {
	return goLoads("rules_dx")
}

func (l *goLang) ApparentLoads(moduleToApparentName func(string) string) []rule.LoadInfo {
	repoName := moduleToApparentName("rules_dx")
	if repoName == "" {
		repoName = "rules_dx"
	}
	return goLoads(repoName)
}

func goLoads(rulesRepo string) []rule.LoadInfo {
	return []rule.LoadInfo{
		{Name: "@" + rulesRepo + "//go/rules:defs.bzl", Symbols: []string{LibraryKind, TestKind}},
	}
}

// Imports indexes one reusable import identity per Go source owned by a
// library rule: the exact module stem of each non-test source. Test-owned
// sources never contribute an identity, and test rules provide no identities
// (tests are never dependencies).
func (*goLang) Imports(_ *config.Config, r *rule.Rule, _ *rule.File) []resolve.ImportSpec {
	if r.Kind() != LibraryKind {
		return nil
	}
	var specs []resolve.ImportSpec
	for _, src := range r.AttrStrings("srcs") {
		if !isSupported(src) || IsTestSource(src) {
			continue
		}
		specs = append(specs, resolve.ImportSpec{Lang: languageName, Imp: ModuleName(src)})
	}
	return specs
}

func (*goLang) Embeds(*rule.Rule, label.Label) []label.Label { return nil }

func (l *goLang) GenerateRules(args language.GenerateArgs) language.GenerateResult {
	return l.generateRules(args)
}

func isSupported(name string) bool {
	for _, ext := range SupportedExts {
		if strings.HasSuffix(name, ext) {
			return true
		}
	}
	return false
}

func (l *goLang) generateRules(args language.GenerateArgs) language.GenerateResult {
	var sources []string
	var testSources []string
	for _, name := range args.RegularFiles {
		if !isSupported(name) {
			continue
		}
		if IsTestSource(name) {
			testSources = append(testSources, name)
			continue
		}
		sources = append(sources, name)
	}
	sort.Strings(sources)
	sort.Strings(testSources)
	if len(sources) == 0 && len(testSources) == 0 {
		return mergeStale(args.File, language.GenerateResult{})
	}
	// Test-only directories have no library to embed: fail closed instead
	// of generating a dangling test or guessing an owner.
	if len(sources) == 0 {
		l.fail("go: %s: test sources %s without non-test sources; add the package sources to this directory or split the tests before adopting generation", args.Rel, strings.Join(testSources, ", "))
		return language.GenerateResult{}
	}

	name, err := DirTargetName(args.Rel)
	if err != nil {
		l.fail("go: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}
	testName := name + "_test"

	packages := make(map[string]bool)
	seen := make(map[string]bool)
	var imports []string
	var libPkg string
	for _, src := range sources {
		content, err := os.ReadFile(filepath.Join(args.Dir, src))
		if err != nil {
			l.fail("go: %s: read %s: %v", args.Rel, src, err)
			continue
		}
		pkg, err := ParsePackage(content)
		if err != nil {
			l.fail("go: %s: parse package %s: %v", args.Rel, src, err)
			continue
		}
		if pkg == "main" {
			l.fail("go: %s: %s declares package main; thin go_binary entries stay handwritten, so split package-main sources into their own directory before adopting generation", args.Rel, src)
			continue
		}
		packages[pkg] = true
		libPkg = pkg
		roots, err := ParseImports(content)
		if err != nil {
			l.fail("go: %s: parse imports %s: %v", args.Rel, src, err)
			continue
		}
		for _, root := range roots {
			if IsCgoImport(root) {
				l.fail("go: %s: %s imports cgo (\"C\"); cgo stays handwritten, so split cgo sources into their own handwritten directory before adopting generation", args.Rel, src)
				continue
			}
			if IsStdLib(root) || seen[root] {
				continue
			}
			seen[root] = true
			imports = append(imports, root)
		}
	}
	if len(l.errors) > 0 {
		return language.GenerateResult{}
	}
	if len(packages) > 1 {
		names := make([]string, 0, len(packages))
		for pkg := range packages {
			names = append(names, pkg)
		}
		sort.Strings(names)
		l.fail("go: %s: mixed packages %s in one directory; split the directory before adopting generation", args.Rel, strings.Join(names, ", "))
		return language.GenerateResult{}
	}
	// Every library package contributes exactly one entry here, so libPkg
	// is set when sources are non-empty and packages are uniform.
	for pkg := range packages {
		libPkg = pkg
	}
	sort.Strings(imports)

	// Package-level test collection: every `*_test.go` in the directory
	// belongs to one `go_test` via `embed`. Internal (`package <lib>`)
	// and external (`package <lib>_test`) forms coexist; any other test
	// package fails closed. Build constraints are preserved by including
	// every test source and letting the toolchain select per platform.
	// Test-only imports resolve onto the test target; the library reaches
	// the test via `embed`, never via an import edge.
	testSeen := make(map[string]bool)
	var testImports []string
	for _, src := range testSources {
		content, err := os.ReadFile(filepath.Join(args.Dir, src))
		if err != nil {
			l.fail("go: %s: read %s: %v", args.Rel, src, err)
			continue
		}
		pkg, err := ParsePackage(content)
		if err != nil {
			l.fail("go: %s: parse package %s: %v", args.Rel, src, err)
			continue
		}
		if pkg != libPkg && pkg != libPkg+"_test" {
			l.fail("go: %s: %s declares package %s, want %s or %s_test; split the directory before adopting generation", args.Rel, src, pkg, libPkg, libPkg)
			continue
		}
		roots, err := ParseImports(content)
		if err != nil {
			l.fail("go: %s: parse imports %s: %v", args.Rel, src, err)
			continue
		}
		for _, root := range roots {
			if IsCgoImport(root) {
				l.fail("go: %s: %s imports cgo (\"C\"); cgo stays handwritten, so split cgo sources into their own handwritten directory before adopting generation", args.Rel, src)
				continue
			}
			if IsStdLib(root) || seen[root] || testSeen[root] {
				continue
			}
			testSeen[root] = true
			testImports = append(testImports, root)
		}
	}
	if len(l.errors) > 0 {
		return language.GenerateResult{}
	}
	sort.Strings(testImports)

	claimants := []Claimant{{Name: name, Source: args.Rel, Kind: LibraryKind}}
	if len(testSources) > 0 {
		claimants = append(claimants, Claimant{Name: testName, Source: args.Rel, Kind: TestKind})
	}
	if err := checkClaims(args.File, args.OtherGen, claimants); err != nil {
		l.fail("go: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}

	result := language.GenerateResult{}
	r := rule.NewRule(LibraryKind, name)
	r.SetAttr("srcs", sources)
	// Pure-Go scope: generated rules carry srcs plus importpath only, never
	// cgo/race scope attrs (cgo, pure, race, msan, gotags, cdeps); those stay
	// handwritten on wrappers that need them.
	if importpath, ok := goImportPath(args.Config.RepoRoot, args.Dir); ok {
		r.SetAttr("importpath", importpath)
	}
	result.Gen = append(result.Gen, r)
	result.Imports = append(result.Imports, targetImports{imports: imports})
	if len(testSources) > 0 {
		t := rule.NewRule(TestKind, testName)
		t.SetAttr("srcs", testSources)
		t.SetAttr("embed", []string{":" + name})
		result.Gen = append(result.Gen, t)
		result.Imports = append(result.Imports, testTargetImports{imports: testImports})
	}
	if isFixturePath(args.Rel) {
		for _, r := range result.Gen {
			r.SetAttr("testonly", true)
		}
	}
	return mergeStale(args.File, result)
}

// checkClaims fails closed on same-package normalized-name collisions: a
// generated rule sharing its name with a handwritten rule of another
// kind fails. A same-kind handwritten owner is ordinary Gazelle merge.
// Handwritten-only duplicates are not ours to judge.
func checkClaims(file *rule.File, other []*rule.Rule, claimants []Claimant) error {
	existing := make(map[string]string)
	if file != nil {
		for _, r := range file.Rules {
			existing[r.Name()] = r.Kind()
		}
	}
	for _, r := range other {
		existing[r.Name()] = r.Kind()
	}
	for _, c := range claimants {
		if have, ok := existing[c.Name]; ok && have != c.Kind {
			return fmt.Errorf("target name %q is claimed by generated %s(%s) and existing %s", c.Name, c.Kind, c.Source, have)
		}
	}
	return nil
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
	if file == nil {
		return result
	}
	for _, existing := range file.Rules {
		if _, owned := goKinds[existing.Kind()]; !owned || desired[existing.Kind()+"\x00"+existing.Name()] {
			continue
		}
		result.Empty = append(result.Empty, rule.NewRule(existing.Kind(), existing.Name()))
	}
	return result
}

// goImportPath derives the rules_go importpath for one generated library
// from the nearest enclosing go.mod: module path plus the slash-separated
// subpath from the module root to dir. It walks up from dir to repoRoot
// (inclusive); when no go.mod is found it reports false and the caller
// omits importpath (source-only fixtures without a module keep their
// current generation-only shape). A found but unparseable go.mod is a
// generation failure at the call site, never a guessed path.
func goImportPath(repoRoot, dir string) (string, bool) {
	cleanDir := filepath.Clean(dir)
	cleanRoot := filepath.Clean(repoRoot)
	for cur := cleanDir; ; cur = filepath.Dir(cur) {
		candidate := filepath.Join(cur, "go.mod")
		if content, err := os.ReadFile(candidate); err == nil {
			module, ok := parseGoModule(string(content))
			if !ok {
				return "", false
			}
			rel, err := filepath.Rel(cur, cleanDir)
			if err != nil {
				return "", false
			}
			slash := filepath.ToSlash(rel)
			if slash == "." || slash == "" {
				return module, true
			}
			return module + "/" + slash, true
		}
		if cur == cleanRoot || cur == filepath.Dir(cur) {
			return "", false
		}
		// Do not walk above the repo root: foreign trees carry their own
		// go.mod; absence means source-only generation without a module.
		if len(cur) < len(cleanRoot) || !strings.HasPrefix(cur, cleanRoot) {
			return "", false
		}
	}
}

// parseGoModule extracts the module path from go.mod content: the first
// `module <path>` line. It reports false when no such line exists so the
// caller can fail closed instead of inventing a path.
func parseGoModule(content string) (string, bool) {
	for _, line := range strings.Split(content, "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "//") {
			continue
		}
		fields := strings.Fields(trimmed)
		if len(fields) >= 2 && fields[0] == "module" {
			path := strings.Trim(fields[1], "\"'")
			if path != "" {
				return path, true
			}
			return "", false
		}
	}
	return "", false
}

func (l *goLang) Resolve(c *config.Config, ix *resolve.RuleIndex, _ *repo.RemoteCache, r *rule.Rule, raw interface{}, from label.Label) {
	var roots []string
	isTest := false
	switch typed := raw.(type) {
	case targetImports:
		if r.Kind() != LibraryKind {
			return
		}
		roots = typed.imports
	case testTargetImports:
		if r.Kind() != TestKind {
			return
		}
		roots = typed.imports
		isTest = true
	default:
		return
	}
	deps := make(map[string]bool)
	for _, name := range roots {
		if IsStdLib(name) {
			continue
		}
		spec := resolve.ImportSpec{Lang: languageName, Imp: name}
		if override, found := resolve.FindRuleWithOverride(c, spec, languageName); found {
			if ignore := matchingIgnore(c, name); ignore != nil {
				ignore.used = true
				l.fail("go: %s: import %q has both an exact resolve mapping and ignore", from, name)
				continue
			}
			// The owning library reaches the package-level test via
			// `embed`, never via an import edge: drop same-package
			// overrides on tests to avoid duplicating the embed.
			if isTest && override.Pkg == from.Pkg {
				continue
			}
			deps[override.Rel(from.Repo, from.Pkg).String()] = true
			continue
		}
		matches := ix.FindRulesByImportWithConfig(c, spec, languageName)
		switch len(matches) {
		case 1:
			if matches[0].Label == from {
				continue
			}
			// Same-package library matches reach the test via `embed`.
			if isTest && matches[0].Label.Pkg == from.Pkg {
				continue
			}
			deps[matches[0].Label.Rel(from.Repo, from.Pkg).String()] = true
		case 0:
			if ignore := matchingIgnore(c, name); ignore != nil {
				ignore.used = true
				continue
			}
			l.fail("go: %s: unresolved import %q; add a local one-source library or an exact # gazelle:resolve mapping", from, name)
		default:
			l.fail("go: %s: ambiguous import %q resolves to %s", from, name, formatMatches(matches))
		}
	}
	if len(deps) == 0 {
		return
	}
	labels := make([]string, 0, len(deps))
	for dep := range deps {
		labels = append(labels, dep)
	}
	sort.Strings(labels)
	r.SetAttr("deps", unionStrings(r.AttrStrings("deps"), labels))
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
	for i := len(raw.(*goConfig).ignores) - 1; i >= 0; i-- {
		if entry := raw.(*goConfig).ignores[i]; entry.value == name {
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

// CollectUsedIgnores reports used dx_ignore_import entries visible in c
// as (path, value) pairs for the composed `//dx:generate` witness.
// See: docs/cli/commands/generate.md.
func CollectUsedIgnores(c *config.Config) [][2]string {
	raw, ok := c.Exts[languageName]
	if !ok || raw == nil {
		return nil
	}
	cfg, ok := raw.(*goConfig)
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
