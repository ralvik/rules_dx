package cc

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

const languageName = "cc"

var ccKinds = map[string]rule.KindInfo{
	LibraryKind: rule.KindInfo{
		MatchAttrs:    []string{"srcs", "hdrs"},
		NonEmptyAttrs: map[string]bool{"srcs": true, "hdrs": true},
		MergeableAttrs: map[string]bool{
			"srcs": true,
			"hdrs": true,
			"deps": true,
		},
		ResolveAttrs: map[string]bool{"deps": true},
	},
}

type ccLang struct {
	language.BaseLang
	errors  []string
	ignores []*ignoreEntry
}

type ccConfig struct {
	ignores []*ignoreEntry
}

type ignoreEntry struct {
	value string
	path  string
	used  bool
}

// targetImports is the deduplicated union of header-basename identities for
// one generated library's non-test sources and headers. Angle includes
// never appear (toolchain-provided, never edges); every quoted identity
// resolves strictly or fails generation.
type targetImports struct {
	imports []string
}

// NewLanguage returns the private first-party C/C++ Gazelle extension.
func NewLanguage() language.Language { return &ccLang{} }

func (l *ccLang) Before(context.Context) { l.errors = nil; l.ignores = nil }

func (l *ccLang) DoneGeneratingRules() {}

func (l *ccLang) RegisterFlags(*flag.FlagSet, string, *config.Config) {}

func (l *ccLang) CheckFlags(*flag.FlagSet, *config.Config) error { return nil }

func (*ccLang) KnownDirectives() []string { return []string{"dx_ignore_import"} }

func (l *ccLang) Configure(c *config.Config, rel string, f *rule.File) {
	var inherited []*ignoreEntry
	if raw, ok := c.Exts[languageName]; ok {
		inherited = append(inherited, raw.(*ccConfig).ignores...)
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
				l.fail("cc: //%s: malformed # gazelle:dx_ignore_import %s", rel, directive.Value)
			}
		}
	}
	c.Exts[languageName] = &ccConfig{ignores: inherited}
}

func (l *ccLang) fail(format string, args ...interface{}) {
	l.errors = append(l.errors, fmt.Sprintf(format, args...))
}

func (l *ccLang) AfterResolvingDeps(context.Context) {
	for _, ignore := range l.ignores {
		if !ignore.used {
			l.fail("cc: //%s: stale # gazelle:dx_ignore_import cc %s matches no literal reference", ignore.path, ignore.value)
		}
	}
	if len(l.errors) == 0 {
		return
	}
	sort.Strings(l.errors)
	panic("CC generation failed:\n" + strings.Join(l.errors, "\n"))
}

func (*ccLang) Name() string { return languageName }

func (*ccLang) Kinds() map[string]rule.KindInfo { return ccKinds }

func (*ccLang) Loads() []rule.LoadInfo {
	return ccLoads("rules_dx")
}

func (l *ccLang) ApparentLoads(moduleToApparentName func(string) string) []rule.LoadInfo {
	repoName := moduleToApparentName("rules_dx")
	if repoName == "" {
		repoName = "rules_dx"
	}
	return ccLoads(repoName)
}

func ccLoads(rulesRepo string) []rule.LoadInfo {
	return []rule.LoadInfo{
		{Name: "@" + rulesRepo + "//cc/rules:defs.bzl", Symbols: []string{LibraryKind}},
	}
}

// Imports indexes one reusable import identity per header owned by a
// library rule: the exact basename of each non-test header. Test-owned
// headers never contribute an identity. Sources contribute no identity:
// importers reference headers, never sources.
func (*ccLang) Imports(_ *config.Config, r *rule.Rule, _ *rule.File) []resolve.ImportSpec {
	if r.Kind() != LibraryKind {
		return nil
	}
	var specs []resolve.ImportSpec
	for _, hdr := range r.AttrStrings("hdrs") {
		if !IsHeader(hdr) || IsTestSource(hdr) {
			continue
		}
		specs = append(specs, resolve.ImportSpec{Lang: languageName, Imp: HeaderIdentity(hdr)})
	}
	return specs
}

func (*ccLang) Embeds(*rule.Rule, label.Label) []label.Label { return nil }

func (l *ccLang) GenerateRules(args language.GenerateArgs) language.GenerateResult {
	return l.generateRules(args)
}

func (l *ccLang) generateRules(args language.GenerateArgs) language.GenerateResult {
	var sources []string
	var headers []string
	for _, name := range args.RegularFiles {
		if IsTestSource(name) {
			continue
		}
		switch {
		case IsSource(name):
			sources = append(sources, name)
		case IsHeader(name):
			headers = append(headers, name)
		}
	}
	sort.Strings(sources)
	sort.Strings(headers)
	if len(sources) == 0 && len(headers) == 0 {
		return mergeStale(args.File, language.GenerateResult{})
	}

	name, err := DirTargetName(args.Rel)
	if err != nil {
		l.fail("cc: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}

	seen := make(map[string]bool)
	var imports []string
	for _, file := range append(append([]string{}, sources...), headers...) {
		content, err := os.ReadFile(filepath.Join(args.Dir, file))
		if err != nil {
			l.fail("cc: %s: read %s: %v", args.Rel, file, err)
			continue
		}
		if IsSource(file) && DefinesMain(content) {
			l.fail("cc: %s: %s defines main; thin cc_binary entries stay handwritten, so split main-defining sources into their own directory before adopting generation", args.Rel, file)
			continue
		}
		for _, root := range ParseQuotedIncludes(content) {
			if seen[root] {
				continue
			}
			seen[root] = true
			imports = append(imports, root)
		}
	}
	if len(l.errors) > 0 {
		return language.GenerateResult{}
	}
	sort.Strings(imports)

	if err := checkClaims(args.File, args.OtherGen, []Claimant{{Name: name, Source: args.Rel, Kind: LibraryKind}}); err != nil {
		l.fail("cc: %s: %v", args.Rel, err)
		return language.GenerateResult{}
	}

	result := language.GenerateResult{}
	r := rule.NewRule(LibraryKind, name)
	if len(sources) > 0 {
		r.SetAttr("srcs", sources)
	}
	if len(headers) > 0 {
		r.SetAttr("hdrs", headers)
	}
	result.Gen = append(result.Gen, r)
	result.Imports = append(result.Imports, targetImports{imports: imports})
	return mergeStale(args.File, result)
}

// checkClaims fails closed on same-package normalized-name collisions: a
// generated library sharing its name with a handwritten rule of another
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
		if have, ok := existing[c.Name]; ok && have != LibraryKind {
			return fmt.Errorf("target name %q is claimed by generated %s(%s) and existing %s", c.Name, LibraryKind, c.Source, have)
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
		if _, owned := ccKinds[existing.Kind()]; !owned || desired[existing.Kind()+"\x00"+existing.Name()] {
			continue
		}
		result.Empty = append(result.Empty, rule.NewRule(existing.Kind(), existing.Name()))
	}
	return result
}

func (l *ccLang) Resolve(c *config.Config, ix *resolve.RuleIndex, _ *repo.RemoteCache, r *rule.Rule, raw interface{}, from label.Label) {
	imports, ok := raw.(targetImports)
	if !ok {
		return
	}
	if r.Kind() != LibraryKind {
		return
	}
	deps := make(map[string]bool)
	for _, name := range imports.imports {
		spec := resolve.ImportSpec{Lang: languageName, Imp: name}
		if override, found := resolve.FindRuleWithOverride(c, spec, languageName); found {
			if ignore := matchingIgnore(c, name); ignore != nil {
				ignore.used = true
				l.fail("cc: %s: import %q has both an exact resolve mapping and ignore", from, name)
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
			l.fail("cc: %s: unresolved import %q; add a local one-header library or an exact # gazelle:resolve mapping", from, name)
		default:
			l.fail("cc: %s: ambiguous import %q resolves to %s", from, name, formatMatches(matches))
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
	for i := len(raw.(*ccConfig).ignores) - 1; i >= 0; i-- {
		if entry := raw.(*ccConfig).ignores[i]; entry.value == name {
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
