// Corpus split generation owned by the first-party Rust Gazelle extension.
//
// Issue #15: each Bazel package's single `corpus` real_source_target mixes
// markdown/starlark/toml under one config binding. The split emits one
// target per populated content type (`corpus_markdown`, `corpus_starlark`,
// `corpus_toml`, `corpus_json`), each carrying exactly its own tool's
// native config, so scoping (`dx lint //docs:corpus_markdown`), config
// changes, and failure attribution stay precise.
//
// Every corpus block is written and maintained by this workflow
// (`dx generate`), enforced by `generate_check` in CI. Hand-maintained
// split targets are never created: the planner owns srcs/hints/tags and
// preserves only the unclassified `markdown_siblings` (link-resolution
// siblings, never linted) from the pre-split `corpus` and any existing
// `corpus_markdown`.
//
// Corpus stays for target-less files only (docs, BUILD files, configs);
// code rides normal targets. Only markdown, starlark (BUILD.bazel,
// MODULE.bazel, *.bzl), toml, and json are collected. Rust/Python/JS and
// other code classes never enter the corpus. Lockfiles (`*.lock`) and the
// paket2bazel hub outputs (generator-owned, never sources) are excluded.
// Files already owned by same-package fixture `real_source_target`s (for
// example `quality/testdata:real_clean.bzl`) stay excluded so fixture
// evidence stays frozen.
package rust

import (
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"

	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/rule"
)

const corpusKind = "real_source_target"

// corpusValeHint is the single Vale configuration every linted Markdown
// corpus binds. Starlark (buildifier) and TOML (taplo) run on pinned
// upstream defaults with no hint; JSON runs on pinned defaults as well.
const corpusValeHint = "//quality:corpus_vale_config"

// corpusTag is the shared tag on every generated corpus split target so CI
// and docs scope without name matching: `attr(tags, corpus, ...)` covers
// `corpus_markdown`, `corpus_starlark`, `corpus_toml`, and `corpus_json`.
const corpusTag = "corpus"

type corpusType struct {
	id   string
	attr string
	name string
}

var corpusTypes = []corpusType{
	{id: "markdown", attr: "markdown_srcs", name: "corpus_markdown"},
	{id: "starlark", attr: "starlark_srcs", name: "corpus_starlark"},
	{id: "toml", attr: "toml_srcs", name: "corpus_toml"},
	{id: "json", attr: "json_srcs", name: "corpus_json"},
}

// corpusKinds declares the managed corpus rule kind: matched by name,
// every planner-owned attr mergeable so fresh lists replace stale ones
// (Gazelle preserves non-mergeable attrs on match, which would pin
// deleted files). This replaces globs (`["BUILD.bazel"] +
// glob(["*.bzl"])`, `glob(["**/*.md"])`) with explicit sorted lists
// instead of attempting expression merges (which fail closed with
// "could not merge expression"). The planner owns srcs/hints/siblings/
// tags outright and preserves siblings explicitly; fixtures use
// non-corpus names and are excluded from collection, never overwritten.
func corpusKinds() map[string]rule.KindInfo {
	return map[string]rule.KindInfo{
		corpusKind: {
			MatchAttrs: []string{"name"},
			// Empty splits (only name/tags, no sources) delete via the
			// generic stale sweep: tags alone never keep a split alive.
			NonEmptyAttrs: map[string]bool{
				"markdown_srcs":     true,
				"starlark_srcs":     true,
				"toml_srcs":         true,
				"json_srcs":         true,
				"markdown_siblings": true,
				"aspect_hints":      true,
			},
			MergeableAttrs: map[string]bool{
				"markdown_srcs":     true,
				"starlark_srcs":     true,
				"toml_srcs":         true,
				"json_srcs":         true,
				"markdown_siblings": true,
				"aspect_hints":      true,
				"tags":              true,
			},
		},
	}
}

// corpusLoads exposes the real_source_target constructor for generated
// corpus targets. Gazelle materializes only the symbols generation uses.
func corpusLoads(rulesRepo string) rule.LoadInfo {
	return rule.LoadInfo{
		Name:    "@" + rulesRepo + "//quality:fixtures.bzl",
		Symbols: []string{"real_source_target"},
	}
}

// corpusPlan is the per-directory split computation: rules to generate
// and legacy single-corpus plus emptied splits to delete.
type corpusPlan struct {
	gen   []*rule.Rule
	empty []*rule.Rule
}

// planCorpus collects target-less files for one Bazel package and emits
// one corpus_* target per populated type. Existing siblings are preserved
// from the legacy `corpus` and any existing `corpus_markdown`; files
// already listed as siblings stay siblings (unclassified, never linted)
// and never duplicate into srcs. Files owned by same-package fixtures
// stay excluded. hasOtherGen reports whether Rust or native-config rules
// will already create a BUILD file (or one exists): a package whose only
// collectable file would be its own BUILD.bazel still gains a starlark
// split when a BUILD will exist anyway, so reruns stay idempotent, but an
// otherwise empty directory without a BUILD file gains nothing.
//
// Only directories that already have a BUILD file (or will gain one via
// Rust/native-config generation) ever gain corpus splits: subdirectories
// without a BUILD are part of the parent package (Bazel package boundary)
// and stay owned by the parent via recursive collection. Creating BUILD
// files for every docs subdirectory would carve new subpackages and break
// the parent's ownership in a single run (parent processed before child
// BUILD exists over-collects, requiring a second run to converge). The
// early return below keeps generation single-run idempotent and prevents
// tool-local trees (`.opencode/`, `node_modules/`) from materializing
// packages.
func planCorpus(args language.GenerateArgs, hasOtherGen bool) *corpusPlan {
	plan := &corpusPlan{}
	rel := args.Rel
	// Tool-local and third-party closures never carry corpus owners:
	// `.opencode/` (agent-local tool state with its own node_modules),
	// any `node_modules/` (pnpm/npm closures, gitignored), and the
	// depcheck truth-table fixtures (intentionally unresolved imports,
	// issue #22) are excluded.
	if rel == ".opencode" || strings.HasPrefix(rel, ".opencode/") {
		return plan
	}
	if rel == "node_modules" || strings.Contains(rel, "/node_modules/") || strings.HasPrefix(rel, "node_modules/") {
		return plan
	}
	if rel == "tools/depcheck/testdata" || strings.HasPrefix(rel, "tools/depcheck/testdata/") {
		return plan
	}
	// Never materialize new packages for tool-local or docs-only dirs:
	// without an existing BUILD and without Rust/native-config rules
	// that will create one, the directory stays owned by its parent
	// (recursive glob/explicit list). This prevents `.opencode/`,
	// `node_modules/`, `docs/` subdirs, `examples/consumer-ci/`, and
	// `gazelle/*/testdata/` from gaining unwanted BUILD files in one run.
	if args.File == nil && !hasOtherGen {
		return plan
	}
	existing := map[string]*rule.Rule{}
	if args.File != nil {
		for _, r := range args.File.Rules {
			existing[r.Name()] = r
		}
	}
	// Fixture-owned files (same-package real_source_target with a
	// non-corpus name) stay excluded so fixture evidence stays frozen.
	fixtureOwned := map[string]bool{}
	for _, r := range existing {
		if r.Kind() != corpusKind {
			continue
		}
		name := r.Name()
		if name == "corpus" || strings.HasPrefix(name, "corpus_") {
			continue
		}
		for _, attr := range []string{"javascript_srcs", "jsx_srcs", "typescript_srcs", "tsx_srcs", "json_srcs", "python_srcs", "python_stub_srcs", "rust_srcs", "starlark_srcs", "toml_srcs", "markdown_srcs"} {
			for _, src := range r.AttrStrings(attr) {
				// Only package-relative explicit files participate;
				// globs and labels never appear on fixtures.
				if src == "" || strings.Contains(src, "*") || strings.HasPrefix(src, "//") || strings.HasPrefix(src, "@") || strings.HasPrefix(src, ":") {
					continue
				}
				fixtureOwned[src] = true
			}
		}
	}
	// Preserve unclassified siblings from the legacy single corpus and
	// any existing markdown split. Sibling labels (cross-package `//`
	// plus package-relative unclassified like CHANGELOG.md/LICENSE)
	// carry forward verbatim, sorted.
	siblingSet := map[string]bool{}
	for _, name := range []string{"corpus", "corpus_markdown"} {
		if r, ok := existing[name]; ok && r.Kind() == corpusKind {
			for _, s := range r.AttrStrings("markdown_siblings") {
				siblingSet[s] = true
			}
		}
	}
	// Local sibling files (package-relative, no label prefix) that are
	// otherwise collectable stay siblings and never duplicate into srcs.
	localSiblings := map[string]bool{}
	for s := range siblingSet {
		if s == "" || strings.HasPrefix(s, "//") || strings.HasPrefix(s, "@") || strings.HasPrefix(s, ":") || strings.Contains(s, "*") {
			continue
		}
		localSiblings[s] = true
	}
	collected := map[string][]string{
		"markdown": {},
		"starlark": {},
		"toml":     {},
		"json":     {},
	}
	seen := map[string]bool{}
	add := func(typ, pkgRel string) {
		if pkgRel == "" || seen[typ+"\x00"+pkgRel] {
			return
		}
		if fixtureOwned[pkgRel] || localSiblings[pkgRel] {
			return
		}
		seen[typ+"\x00"+pkgRel] = true
		collected[typ] = append(collected[typ], pkgRel)
	}
	// Recursive walk: subdirectories with their own BUILD.bazel/BUILD
	// are separate Bazel packages and never enter the parent corpus.
	// Hidden dirs, bazel-* output trees, and node_modules closures are
	// skipped like the Rust source walk.
	root := args.Dir
	_ = filepath.WalkDir(root, func(name string, entry os.DirEntry, err error) error {
		if err != nil {
			return nil
		}
		if entry.IsDir() {
			if name == root {
				return nil
			}
			base := entry.Name()
			if strings.HasPrefix(base, ".") || base == "bazel-bin" || base == "bazel-out" || base == "bazel-testlogs" || base == "node_modules" {
				return filepath.SkipDir
			}
			if _, statErr := os.Stat(filepath.Join(name, "BUILD.bazel")); statErr == nil {
				return filepath.SkipDir
			}
			if _, statErr := os.Stat(filepath.Join(name, "BUILD")); statErr == nil {
				return filepath.SkipDir
			}
			return nil
		}
		local, relErr := filepath.Rel(root, name)
		if relErr != nil {
			return nil
		}
		slash := filepath.ToSlash(local)
		// Generator-owned paket hub outputs are never corpus sources.
		if slash == "paket.main.bzl" || slash == "paket.main_extension.bzl" {
			if rel == "third_party/dotnet/deps" {
				return nil
			}
		}
		if strings.HasSuffix(slash, ".lock") {
			return nil
		}
		base := path.Base(slash)
		// Dotfiles are tool-owned configs or hidden state, never linted
		// corpus sources (for example .buildifier.json, .vale.ini).
		// BUILD.bazel/MODULE.bazel never start with a dot, so they stay.
		if strings.HasPrefix(base, ".") {
			return nil
		}
		// Local overrides (for example AGENTS.local.md, dx.local.toml)
		// are gitignored personal overlays, never checked-in corpus
		// sources.
		if strings.Contains(slash, ".local.") {
			return nil
		}
		switch {
		case base == "BUILD.bazel" || base == "MODULE.bazel" || strings.HasSuffix(base, ".bzl"):
			add("starlark", slash)
		case strings.HasSuffix(base, ".md"):
			add("markdown", slash)
		case strings.HasSuffix(base, ".toml"):
			add("toml", slash)
		case strings.HasSuffix(base, ".json"):
			// JSON is preserve-only (issue #15 split dimensions are
			// markdown/starlark/toml): existing package.json/tsconfig
			// ownership carries forward, but new JSON files (locks,
			// tool manifests, scaffold outputs) never auto-enter the
			// corpus. See the preserve block after the walk.
			return nil
		}
		return nil
	})
	// JSON preserve-only: carry forward existing json_srcs from the legacy
	// single corpus and any existing corpus_json split. New JSON files
	// never auto-enter (locks, tool closures, and scaffold outputs stay
	// out); deletions still sweep via the generic stale pass.
	for _, name := range []string{"corpus", "corpus_json"} {
		if r, ok := existing[name]; ok && r.Kind() == corpusKind {
			for _, src := range r.AttrStrings("json_srcs") {
				if src == "" || strings.Contains(src, "*") || strings.HasPrefix(src, "//") || strings.HasPrefix(src, "@") || strings.HasPrefix(src, ":") {
					continue
				}
				if fixtureOwned[src] || localSiblings[src] {
					continue
				}
				if !seen["json\x00"+src] {
					seen["json\x00"+src] = true
					collected["json"] = append(collected["json"], src)
				}
			}
		}
	}
	for _, typ := range collected {
		sort.Strings(typ)
	}
	// Every package owns its own BUILD file: ensure BUILD.bazel is in the
	// starlark split even when it does not exist yet (first generation
	// creates it). Without this the first run omits the split and the
	// second run adds it, breaking idempotency.
	if !seen["starlark\x00BUILD.bazel"] {
		if _, ok := fixtureOwned["BUILD.bazel"]; !ok {
			if _, ok := localSiblings["BUILD.bazel"]; !ok {
				collected["starlark"] = append(collected["starlark"], "BUILD.bazel")
				sort.Strings(collected["starlark"])
			}
		}
	}
	siblings := make([]string, 0, len(siblingSet))
	for s := range siblingSet {
		siblings = append(siblings, s)
	}
	sort.Strings(siblings)
	// An otherwise empty directory without a BUILD file gains nothing:
	// creating a BUILD whose only content is its own self-ownership would
	// materialize packages in every empty dir. When other rules will
	// create the BUILD anyway (or one already exists, or other corpus
	// types/siblings exist), the self-owned starlark split stays.
	onlySelf := len(collected["markdown"]) == 0 && len(siblings) == 0 &&
		len(collected["toml"]) == 0 && len(collected["json"]) == 0 &&
		len(collected["starlark"]) == 1 && collected["starlark"][0] == "BUILD.bazel"
	if onlySelf && args.File == nil && !hasOtherGen {
		return plan
	}
	// Emit one target per populated type. Starlark always has at least
	// BUILD.bazel (the package file itself) except for synthetic test
	// dirs without one; skip empty splits so removal stubs delete them.
	emit := func(typ, attr string, srcs []string) {
		if len(srcs) == 0 {
			return
		}
		r := rule.NewRule(corpusKind, "corpus_"+typ)
		r.SetAttr(attr, srcs)
		r.SetAttr("tags", []string{corpusTag})
		if typ == "markdown" {
			r.SetAttr("aspect_hints", []string{corpusValeHint})
			if len(siblings) > 0 {
				r.SetAttr("markdown_siblings", siblings)
			}
		}
		plan.gen = append(plan.gen, r)
	}
	emit("markdown", "markdown_srcs", collected["markdown"])
	// Markdown siblings without markdown sources still need a home: a
	// siblings-only markdown split (no hints, no srcs) preserves the
	// link-resolution closure without linting anything.
	if len(collected["markdown"]) == 0 && len(siblings) > 0 {
		r := rule.NewRule(corpusKind, "corpus_markdown")
		r.SetAttr("markdown_siblings", siblings)
		r.SetAttr("tags", []string{corpusTag})
		plan.gen = append(plan.gen, r)
	}
	emit("starlark", "starlark_srcs", collected["starlark"])
	emit("toml", "toml_srcs", collected["toml"])
	emit("json", "json_srcs", collected["json"])
	// Legacy `corpus` deletes explicitly alongside the generic stale sweep
	// in mergeStale (desired holds only the fresh splits): the explicit
	// Empty guarantees single-run migration even if the stale pass is
	// bypassed, while the generic pass stays as the single source for
	// stale `corpus_*` splits (duplicates are harmless, Gazelle dedups).
	if _, ok := existing["corpus"]; ok {
		if r, ok := existing["corpus"]; ok && r.Kind() == corpusKind {
			plan.empty = append(plan.empty, rule.NewRule(corpusKind, "corpus"))
		}
	}
	return plan
}

// applyCorpusHints is a no-op placeholder: corpus splits already carry
// exactly their own tool's native config (markdown binds Vale, others run
// pinned defaults), so Rust rules never inherit corpus hints.
func applyCorpusHints() {}
