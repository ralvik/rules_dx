package javascript

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

const (
	languageName = "javascript"
	libraryKind  = "dx_js_library"
	testKind     = "dx_js_test"
	binaryKind   = "dx_js_binary"
	// rootNodeModules is the default importer facade for generated tests.
	// Source-only tests still need a Jest runtime; the root importer proves
	// the provider-derived layout first. Per-importer facades arrive with
	// pnpm importer-scope resolution; the attribute stays mergeable so
	// handwritten narrowing survives.
	rootNodeModules = "//:node_modules"
)

var javascriptKinds = map[string]rule.KindInfo{
	libraryKind: libraryKindInfo(),
	testKind:    testKindInfo(),
	binaryKind:  binaryKindInfo(),
}

func libraryKindInfo() rule.KindInfo {
	return rule.KindInfo{
		MatchAttrs:    []string{"srcs"},
		NonEmptyAttrs: map[string]bool{"srcs": true},
		MergeableAttrs: map[string]bool{
			"srcs": true,
			"deps": true,
		},
		ResolveAttrs: map[string]bool{"deps": true},
	}
}

func testKindInfo() rule.KindInfo {
	return rule.KindInfo{
		MatchAttrs:    []string{"srcs"},
		NonEmptyAttrs: map[string]bool{"srcs": true},
		MergeableAttrs: map[string]bool{
			"srcs":         true,
			"data":         true,
			"node_modules": true,
		},
		ResolveAttrs: map[string]bool{"data": true},
	}
}

// binaryKindInfo matches thin binaries by entry_point: a binary owns no
// srcs, so srcs must stay out of MatchAttrs/NonEmptyAttrs or Gazelle would
// delete generated binaries as empty on every merge.
func binaryKindInfo() rule.KindInfo {
	return rule.KindInfo{
		MatchAttrs: []string{"entry_point"},
		MergeableAttrs: map[string]bool{
			"data":         true,
			"node_modules": true,
		},
		ResolveAttrs: map[string]bool{"data": true},
	}
}

type javascriptLang struct {
	language.BaseLang
	errors  []string
	ignores []*ignoreEntry
}

type javascriptConfig struct {
	ignores []*ignoreEntry
}

type ignoreEntry struct {
	value string
	path  string
	used  bool
}

// targetImports is the deduplicated union of literal specifier roots for one
// generated rule's sources. Standard-library roots are dropped at
// collection; every other root resolves strictly or fails generation.
type targetImports struct {
	imports []string
}

// NewLanguage returns the private first-party JavaScript Gazelle extension.
func NewLanguage() language.Language { return &javascriptLang{} }

func (l *javascriptLang) Before(context.Context) { l.errors = nil; l.ignores = nil }

func (l *javascriptLang) DoneGeneratingRules() {}

func (l *javascriptLang) RegisterFlags(*flag.FlagSet, string, *config.Config) {}

func (l *javascriptLang) CheckFlags(*flag.FlagSet, *config.Config) error { return nil }

func (*javascriptLang) KnownDirectives() []string { return []string{"dx_ignore_import"} }

func (l *javascriptLang) Configure(c *config.Config, rel string, f *rule.File) {
	var inherited []*ignoreEntry
	if raw, ok := c.Exts[languageName]; ok {
		inherited = append(inherited, raw.(*javascriptConfig).ignores...)
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
				l.fail("javascript: //%s: malformed # gazelle:dx_ignore_import %s", rel, directive.Value)
			}
		}
	}
	c.Exts[languageName] = &javascriptConfig{ignores: inherited}
}

func (l *javascriptLang) fail(format string, args ...interface{}) {
	l.errors = append(l.errors, fmt.Sprintf(format, args...))
}

func (l *javascriptLang) AfterResolvingDeps(context.Context) {
	for _, ignore := range l.ignores {
		if !ignore.used {
			l.fail("javascript: //%s: stale # gazelle:dx_ignore_import javascript %s matches no literal reference", ignore.path, ignore.value)
		}
	}
	if len(l.errors) == 0 {
		return
	}
	sort.Strings(l.errors)
	panic("JavaScript generation failed:\n" + strings.Join(l.errors, "\n"))
}

func (*javascriptLang) Name() string { return languageName }

func (*javascriptLang) Kinds() map[string]rule.KindInfo { return javascriptKinds }

func (*javascriptLang) Loads() []rule.LoadInfo {
	return javascriptLoads("rules_dx")
}

func (l *javascriptLang) ApparentLoads(moduleToApparentName func(string) string) []rule.LoadInfo {
	repoName := moduleToApparentName("rules_dx")
	if repoName == "" {
		repoName = "rules_dx"
	}
	return javascriptLoads(repoName)
}

func javascriptLoads(rulesRepo string) []rule.LoadInfo {
	return []rule.LoadInfo{
		{Name: "@" + rulesRepo + "//javascript/rules:defs.bzl", Symbols: []string{libraryKind, testKind, binaryKind}},
	}
}

// Imports indexes one reusable import identity per JavaScript source owned
// by a library rule: the exact module stem. Test rules are leaves and
// provide nothing.
func (*javascriptLang) Imports(_ *config.Config, r *rule.Rule, _ *rule.File) []resolve.ImportSpec {
	if r.Kind() != libraryKind {
		return nil
	}
	var specs []resolve.ImportSpec
	for _, src := range r.AttrStrings("srcs") {
		if !isSupported(src) {
			continue
		}
		specs = append(specs, resolve.ImportSpec{Lang: languageName, Imp: ModuleName(src)})
	}
	return specs
}

func (*javascriptLang) Embeds(*rule.Rule, label.Label) []label.Label { return nil }

func (l *javascriptLang) GenerateRules(args language.GenerateArgs) language.GenerateResult {
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

func (l *javascriptLang) generateRules(args language.GenerateArgs) language.GenerateResult {
	var sources []string
	for _, name := range args.RegularFiles {
		if isSupported(name) {
			sources = append(sources, name)
		}
	}
	sort.Strings(sources)
	if len(sources) == 0 {
		return mergeStale(args.File, language.GenerateResult{})
	}

	type plan struct {
		name    string
		src     string
		test    bool
		entry   bool
		imports []string
	}
	var plans []plan
	for _, src := range sources {
		content, err := os.ReadFile(filepath.Join(args.Dir, src))
		if err != nil {
			l.fail("javascript: %s: read %s: %v", args.Rel, src, err)
			continue
		}
		name, err := TargetName(src)
		if err != nil {
			l.fail("javascript: %s: %v", args.Rel, err)
			continue
		}
		p := plan{name: name, src: src, test: IsTestFile(src)}
		p.entry = !p.test && IsEntryFile(src)
		seen := make(map[string]bool)
		for _, root := range ParseImports(content) {
			if IsStdLib(root) || seen[root] {
				continue
			}
			seen[root] = true
			p.imports = append(p.imports, root)
		}
		sort.Strings(p.imports)
		plans = append(plans, p)
	}
	if len(l.errors) > 0 {
		return language.GenerateResult{}
	}

	claimants := make([]Claimant, 0, len(plans)*2)
	for _, p := range plans {
		kind := libraryKind
		if p.test {
			kind = testKind
		}
		claimants = append(claimants, Claimant{Name: p.name, Source: p.src, Kind: kind})
		if p.entry {
			claimants = append(claimants, Claimant{Name: EntryBinaryName(p.name), Source: p.src, Kind: binaryKind})
		}
	}
	if err := checkClaims(args.File, args.OtherGen, claimants); err != nil {
		l.fail("javascript: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}

	var result language.GenerateResult
	for _, p := range plans {
		if p.test {
			r := rule.NewRule(testKind, p.name)
			r.SetAttr("srcs", []string{p.src})
			r.SetAttr("node_modules", rootNodeModules)
			result.Gen = append(result.Gen, r)
			result.Imports = append(result.Imports, targetImports{imports: append([]string(nil), p.imports...)})
			continue
		}
		r := rule.NewRule(libraryKind, p.name)
		r.SetAttr("srcs", []string{p.src})
		result.Gen = append(result.Gen, r)
		result.Imports = append(result.Imports, targetImports{imports: append([]string(nil), p.imports...)})
		if p.entry {
			bin := rule.NewRule(binaryKind, EntryBinaryName(p.name))
			bin.SetAttr("entry_point", p.src)
			bin.SetAttr("data", []string{":" + p.name})
			result.Gen = append(result.Gen, bin)
			result.Imports = append(result.Imports, targetImports{})
		}
	}
	return mergeStale(args.File, result)
}

// claimKind returns the generated rule kind for one claimant: the explicit
// Kind when set (thin-binary claims), otherwise inferred from the source
// (test when IsTestFile, else library).
func claimKind(c Claimant) string {
	if c.Kind != "" {
		return c.Kind
	}
	if IsTestFile(c.Source) {
		return testKind
	}
	return libraryKind
}

// checkClaims fails closed on same-package normalized-name collisions:
// two generated sources claiming one name fail with every claimant,
// including any handwritten owner. A single generated claimant sharing a
// name with a handwritten rule of the same kind is ordinary Gazelle merge;
// a kind mismatch fails. Handwritten-only duplicates are not ours to judge.
func checkClaims(file *rule.File, other []*rule.Rule, claimants []Claimant) error {
	byName := make(map[string][]string, len(claimants))
	order := make([]string, 0, len(claimants))
	for _, c := range claimants {
		if _, ok := byName[c.Name]; !ok {
			order = append(order, c.Name)
		}
		byName[c.Name] = append(byName[c.Name], c.Source)
	}
	existing := make(map[string]string)
	if file != nil {
		for _, r := range file.Rules {
			existing[r.Name()] = r.Kind()
		}
	}
	for _, r := range other {
		existing[r.Name()] = r.Kind()
	}
	for _, name := range order {
		sources := byName[name]
		kind := ""
		for _, p := range claimants {
			if p.Name == name {
				kind = claimKind(p)
				break
			}
		}
		if len(sources) > 1 {
			all := append([]string(nil), sources...)
			if have, ok := existing[name]; ok {
				all = append(all, "handwritten:"+have+":"+name)
			}
			return &CollisionError{Name: name, Claimants: all}
		}
		if have, ok := existing[name]; ok && have != kind {
			return fmt.Errorf("target name %q is claimed by generated %s(%s) and existing %s", name, kind, sources[0], have)
		}
	}
	return nil
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
		if _, owned := javascriptKinds[existing.Kind()]; !owned || desired[existing.Kind()+"\x00"+existing.Name()] {
			continue
		}
		result.Empty = append(result.Empty, rule.NewRule(existing.Kind(), existing.Name()))
	}
	return result
}

func (l *javascriptLang) Resolve(c *config.Config, ix *resolve.RuleIndex, _ *repo.RemoteCache, r *rule.Rule, raw interface{}, from label.Label) {
	imports, ok := raw.(targetImports)
	if !ok {
		return
	}
	// Thin binaries carry only entry metadata; their library owns the graph.
	if r.Kind() == binaryKind {
		return
	}
	resolveAttr := "deps"
	if r.Kind() == testKind {
		resolveAttr = "data"
	}
	deps := make(map[string]bool)
	for _, name := range imports.imports {
		if IsStdLib(name) {
			continue
		}
		spec := resolve.ImportSpec{Lang: languageName, Imp: name}
		if override, found := resolve.FindRuleWithOverride(c, spec, languageName); found {
			if ignore := matchingIgnore(c, name); ignore != nil {
				ignore.used = true
				l.fail("javascript: %s: import %q has both an exact resolve mapping and ignore", from, name)
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
			l.fail("javascript: %s: unresolved import %q; add a local one-source library or an exact # gazelle:resolve mapping", from, name)
		default:
			l.fail("javascript: %s: ambiguous import %q resolves to %s", from, name, formatMatches(matches))
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
	if r.Attr(resolveAttr) == nil {
		// Tests already carry user data; libraries start empty. Union with
		// any existing value so handwritten resources survive.
		r.SetAttr(resolveAttr, unionStrings(r.AttrStrings(resolveAttr), labels))
		return
	}
	r.SetAttr(resolveAttr, unionStrings(r.AttrStrings(resolveAttr), labels))
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
	for i := len(raw.(*javascriptConfig).ignores) - 1; i >= 0; i-- {
		if entry := raw.(*javascriptConfig).ignores[i]; entry.value == name {
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
