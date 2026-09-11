package python

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
	languageName = "python"
	libraryKind  = "dx_py_library"
	testKind     = "dx_py_test"
	// importsAttr is the conventional source-only import root. Every
	// generated rule sets imports = ["."] so the owning package directory
	// resolves first-party siblings exactly like the handwritten seed
	// fixtures; it is mergeable so handwritten widening survives.
	importsAttr = "."
)

var pythonKinds = map[string]rule.KindInfo{
	libraryKind: kindInfo(),
	testKind:    kindInfo(),
}

func kindInfo() rule.KindInfo {
	return rule.KindInfo{
		MatchAttrs:    []string{"srcs"},
		NonEmptyAttrs: map[string]bool{"srcs": true},
		MergeableAttrs: map[string]bool{
			"srcs":    true,
			"imports": true,
			"deps":    true,
		},
		ResolveAttrs: map[string]bool{"deps": true},
	}
}

type pythonLang struct {
	language.BaseLang
	errors []string
}

// targetImports is the deduplicated union of literal import roots for one
// generated rule's sources. Standard-library roots are dropped at
// collection; every other root resolves strictly or fails generation.
type targetImports struct {
	imports []string
}

// NewLanguage returns the private first-party Python Gazelle extension.
func NewLanguage() language.Language { return &pythonLang{} }

func (l *pythonLang) Before(context.Context) { l.errors = nil }

func (l *pythonLang) DoneGeneratingRules() {}

func (l *pythonLang) RegisterFlags(*flag.FlagSet, string, *config.Config) {}

func (l *pythonLang) CheckFlags(*flag.FlagSet, *config.Config) error { return nil }

func (*pythonLang) KnownDirectives() []string { return nil }

func (l *pythonLang) Configure(*config.Config, string, *rule.File) {}

func (l *pythonLang) fail(format string, args ...interface{}) {
	l.errors = append(l.errors, fmt.Sprintf(format, args...))
}

func (l *pythonLang) AfterResolvingDeps(context.Context) {
	if len(l.errors) == 0 {
		return
	}
	sort.Strings(l.errors)
	panic("Python generation failed:\n" + strings.Join(l.errors, "\n"))
}

func (*pythonLang) Name() string { return languageName }

func (*pythonLang) Kinds() map[string]rule.KindInfo { return pythonKinds }

func (*pythonLang) Loads() []rule.LoadInfo {
	return pythonLoads("rules_dx")
}

func (l *pythonLang) ApparentLoads(moduleToApparentName func(string) string) []rule.LoadInfo {
	repoName := moduleToApparentName("rules_dx")
	if repoName == "" {
		repoName = "rules_dx"
	}
	return pythonLoads(repoName)
}

func pythonLoads(rulesRepo string) []rule.LoadInfo {
	return []rule.LoadInfo{
		{Name: "@" + rulesRepo + "//python/rules:defs.bzl", Symbols: []string{libraryKind, testKind}},
	}
}

// Imports indexes one reusable import identity per `.py` source owned by a
// library rule: the exact module stem. Test rules are leaves and provide
// nothing; paired `.pyi` stubs never provide an identity.
func (*pythonLang) Imports(_ *config.Config, r *rule.Rule, _ *rule.File) []resolve.ImportSpec {
	if r.Kind() != libraryKind {
		return nil
	}
	var specs []resolve.ImportSpec
	for _, src := range r.AttrStrings("srcs") {
		if !strings.HasSuffix(src, ".py") {
			continue
		}
		specs = append(specs, resolve.ImportSpec{Lang: languageName, Imp: ModuleName(src)})
	}
	return specs
}

func (*pythonLang) Embeds(*rule.Rule, label.Label) []label.Label { return nil }

func (l *pythonLang) GenerateRules(args language.GenerateArgs) language.GenerateResult {
	return l.generateRules(args)
}

func (l *pythonLang) generateRules(args language.GenerateArgs) language.GenerateResult {
	var sources []string
	stubs := make(map[string]bool)
	for _, name := range args.RegularFiles {
		switch {
		case strings.HasSuffix(name, ".py"):
			sources = append(sources, name)
		case strings.HasSuffix(name, ".pyi"):
			stubs[name] = true
		}
	}
	sort.Strings(sources)
	if len(sources) == 0 {
		return mergeStale(args.File, language.GenerateResult{})
	}

	type plan struct {
		name    string
		src     string
		stub    string
		test    bool
		imports []string
	}
	var plans []plan
	for _, src := range sources {
		content, err := os.ReadFile(filepath.Join(args.Dir, src))
		if err != nil {
			l.fail("python: %s: read %s: %v", args.Rel, src, err)
			continue
		}
		name, err := TargetName(src)
		if err != nil {
			l.fail("python: %s: %v", args.Rel, err)
			continue
		}
		p := plan{name: name, src: src, test: IsTestFile(src)}
		if stub := strings.TrimSuffix(src, ".py") + ".pyi"; stubs[stub] {
			p.stub = stub
		}
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

	claimants := make([]Claimant, 0, len(plans))
	for _, p := range plans {
		claimants = append(claimants, Claimant{Name: p.name, Source: p.src})
	}
	if err := checkClaims(args.File, args.OtherGen, claimants); err != nil {
		l.fail("python: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}

	var result language.GenerateResult
	for _, p := range plans {
		kind := libraryKind
		if p.test {
			kind = testKind
		}
		r := rule.NewRule(kind, p.name)
		srcs := []string{p.src}
		if p.stub != "" {
			srcs = append(srcs, p.stub)
		}
		r.SetAttr("srcs", srcs)
		r.SetAttr("imports", []string{importsAttr})
		result.Gen = append(result.Gen, r)
		result.Imports = append(result.Imports, targetImports{imports: append([]string(nil), p.imports...)})
	}
	return mergeStale(args.File, result)
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
				if IsTestFile(p.Source) {
					kind = testKind
				} else {
					kind = libraryKind
				}
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
		if _, owned := pythonKinds[existing.Kind()]; !owned || desired[existing.Kind()+"\x00"+existing.Name()] {
			continue
		}
		result.Empty = append(result.Empty, rule.NewRule(existing.Kind(), existing.Name()))
	}
	return result
}

func (l *pythonLang) Resolve(c *config.Config, ix *resolve.RuleIndex, _ *repo.RemoteCache, r *rule.Rule, raw interface{}, from label.Label) {
	imports, ok := raw.(targetImports)
	if !ok {
		return
	}
	deps := make(map[string]bool)
	for _, name := range imports.imports {
		if IsStdLib(name) {
			continue
		}
		spec := resolve.ImportSpec{Lang: languageName, Imp: name}
		if override, found := resolve.FindRuleWithOverride(c, spec, languageName); found {
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
			l.fail("python: %s: unresolved import %q; add a local one-source library or an exact # gazelle:resolve mapping", from, name)
		default:
			l.fail("python: %s: ambiguous import %q resolves to %s", from, name, formatMatches(matches))
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
	if r.Attr("deps") == nil {
		r.SetAttr("deps", labels)
		return
	}
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

func formatMatches(matches []resolve.FindResult) string {
	labels := make([]string, 0, len(matches))
	for _, match := range matches {
		labels = append(labels, match.Label.String())
	}
	sort.Strings(labels)
	return fmt.Sprintf("[%s]", strings.Join(labels, ", "))
}
