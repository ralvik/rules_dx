package csharp

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

const languageName = "csharp"

var csharpKinds = map[string]rule.KindInfo{
	LibraryKind: rule.KindInfo{
		MatchAttrs:    []string{"srcs"},
		NonEmptyAttrs: map[string]bool{"srcs": true},
		MergeableAttrs: map[string]bool{
			"srcs": true,
			"deps": true,
		},
		ResolveAttrs: map[string]bool{"deps": true},
	},
}

type csharpLang struct {
	language.BaseLang
	errors  []string
	ignores []*ignoreEntry
}

type csharpConfig struct {
	ignores []*ignoreEntry
}

type ignoreEntry struct {
	value string
	path  string
	used  bool
}

type targetImports struct {
	imports []string
}

func NewLanguage() language.Language { return &csharpLang{} }

func (l *csharpLang) Before(context.Context) { l.errors = nil; l.ignores = nil }

func (l *csharpLang) DoneGeneratingRules() {}

func (l *csharpLang) RegisterFlags(*flag.FlagSet, string, *config.Config) {}

func (l *csharpLang) CheckFlags(*flag.FlagSet, *config.Config) error { return nil }

func (*csharpLang) KnownDirectives() []string { return []string{"dx_ignore_import"} }

func (l *csharpLang) Configure(c *config.Config, rel string, f *rule.File) {
	var inherited []*ignoreEntry
	if raw, ok := c.Exts[languageName]; ok {
		inherited = append(inherited, raw.(*csharpConfig).ignores...)
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
				l.fail("csharp: //%s: malformed # gazelle:dx_ignore_import %s", rel, directive.Value)
			}
		}
	}
	c.Exts[languageName] = &csharpConfig{ignores: inherited}
}

func (l *csharpLang) fail(format string, args ...interface{}) {
	l.errors = append(l.errors, fmt.Sprintf(format, args...))
}

func (l *csharpLang) AfterResolvingDeps(context.Context) {
	for _, ignore := range l.ignores {
		if !ignore.used {
			l.fail("csharp: //%s: stale # gazelle:dx_ignore_import csharp %s matches no literal reference", ignore.path, ignore.value)
		}
	}
	if len(l.errors) == 0 {
		return
	}
	sort.Strings(l.errors)
	panic("CSharp generation failed:\n" + strings.Join(l.errors, "\n"))
}

func (*csharpLang) Name() string { return languageName }

func (*csharpLang) Kinds() map[string]rule.KindInfo { return csharpKinds }

func (*csharpLang) Loads() []rule.LoadInfo {
	return csharpLoads("rules_dx")
}

func (l *csharpLang) ApparentLoads(moduleToApparentName func(string) string) []rule.LoadInfo {
	repoName := moduleToApparentName("rules_dx")
	if repoName == "" {
		repoName = "rules_dx"
	}
	return csharpLoads(repoName)
}

func csharpLoads(rulesRepo string) []rule.LoadInfo {
	return []rule.LoadInfo{
		{Name: "@" + rulesRepo + "//csharp/rules:defs.bzl", Symbols: []string{LibraryKind}},
	}
}

func (*csharpLang) Imports(_ *config.Config, r *rule.Rule, _ *rule.File) []resolve.ImportSpec {
	if r.Kind() != LibraryKind {
		return nil
	}
	var specs []resolve.ImportSpec
	for _, src := range r.AttrStrings("srcs") {
		if !isSupported(src) || IsTestSource(src) {
			continue
		}
		specs = append(specs, resolve.ImportSpec{Lang: languageName, Imp: ClassIdentity(src)})
	}
	return specs
}

func (*csharpLang) Embeds(*rule.Rule, label.Label) []label.Label { return nil }

func (l *csharpLang) GenerateRules(args language.GenerateArgs) language.GenerateResult {
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

func (l *csharpLang) generateRules(args language.GenerateArgs) language.GenerateResult {
	var sources []string
	for _, name := range args.RegularFiles {
		if isSupported(name) && !IsTestSource(name) {
			sources = append(sources, name)
		}
	}
	sort.Strings(sources)
	if len(sources) == 0 {
		return mergeStale(args.File, language.GenerateResult{})
	}

	name, err := DirTargetName(args.Rel)
	if err != nil {
		l.fail("csharp: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}

	packages := make(map[string]bool)
	seen := make(map[string]bool)
	var imports []string
	for _, src := range sources {
		content, err := os.ReadFile(filepath.Join(args.Dir, src))
		if err != nil {
			l.fail("csharp: %s: read %s: %v", args.Rel, src, err)
			continue
		}
		if DefinesMain(content) {
			l.fail("csharp: %s: %s defines main; thin csharp_binary entries stay handwritten, so split main-bearing sources into their own directory before adopting generation", args.Rel, src)
			continue
		}
		pkg, err := ParsePackage(content)
		if err != nil {
			l.fail("csharp: %s: parse package %s: %v", args.Rel, src, err)
			continue
		}
		packages[pkg] = true
		for _, root := range ParseImports(content) {
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
		l.fail("csharp: %s: mixed packages %s in one directory; split the directory before adopting generation", args.Rel, strings.Join(names, ", "))
		return language.GenerateResult{}
	}
	sort.Strings(imports)

	if err := checkClaims(args.File, args.OtherGen, []Claimant{{Name: name, Source: args.Rel, Kind: LibraryKind}}); err != nil {
		l.fail("csharp: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}

	result := language.GenerateResult{}
	r := rule.NewRule(LibraryKind, name)
	r.SetAttr("srcs", sources)
	result.Gen = append(result.Gen, r)
	result.Imports = append(result.Imports, targetImports{imports: imports})
	if isFixturePath(args.Rel) {
		for _, r := range result.Gen {
			r.SetAttr("testonly", true)
		}
	}
	return mergeStale(args.File, result)
}

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
		if have, ok := existing[c.Name]; ok && have != LibraryKind {
			return fmt.Errorf("target name %q is claimed by generated %s(%s) and existing %s", c.Name, LibraryKind, c.Source, have)
		}
	}
	return nil
}

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
		if _, owned := csharpKinds[existing.Kind()]; !owned || desired[existing.Kind()+"\x00"+existing.Name()] {
			continue
		}
		result.Empty = append(result.Empty, rule.NewRule(existing.Kind(), existing.Name()))
	}
	return result
}

func (l *csharpLang) Resolve(c *config.Config, ix *resolve.RuleIndex, _ *repo.RemoteCache, r *rule.Rule, raw interface{}, from label.Label) {
	imports, ok := raw.(targetImports)
	if !ok {
		return
	}
	if r.Kind() != LibraryKind {
		return
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
				l.fail("csharp: %s: import %q has both an exact resolve mapping and ignore", from, name)
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
			l.fail("csharp: %s: unresolved import %q; add a local one-source library or an exact # gazelle:resolve mapping", from, name)
		default:
			l.fail("csharp: %s: ambiguous import %q resolves to %s", from, name, formatMatches(matches))
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
	for i := len(raw.(*csharpConfig).ignores) - 1; i >= 0; i-- {
		if entry := raw.(*csharpConfig).ignores[i]; entry.value == name {
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

func CollectUsedIgnores(c *config.Config) [][2]string {
	raw, ok := c.Exts[languageName]
	if !ok || raw == nil {
		return nil
	}
	cfg, ok := raw.(*csharpConfig)
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
