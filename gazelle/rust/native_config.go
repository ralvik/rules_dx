// Native-config generation owned by the first-party Rust Gazelle extension.
//
// The extension recognizes tool-owned config files by exact basename,
// manages one typed native-config target per recognized file, and binds
// Rust rules to their configs through direct aspect_hints entries. The
// recognized filenames are frozen: renaming a tool file is a contract
// change documented in docs/quality/native-configuration.md, never a
// silent behavior drift.
//
// Hand-authored config targets always win: an existing target of the same
// config kind suppresses generation (its name feeds hint resolution) and
// is never stubbed. Only rules the generator emitted (default name plus
// the recognized src) are ever removed, and only once their file or
// selection is gone.
package rust

import (
	"fmt"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/rule"
)

// nativeToolsDirective selects the managed native-config tool set per
// directory: `# gazelle:dx_native_tools <tool...>` with tool ids from the
// frozen table below. The nearest directive wins and inherits otherwise;
// absent means every tool. Unknown ids and bare directives fail before
// BUILD emission.
const nativeToolsDirective = "dx_native_tools"

// nativeTool describes one managed native-config tool: the exact config
// filename it owns, the config rule kind the generator manages, and
// whether owned Rust rules bind to it through aspect_hints. Only tools
// whose configs a Rust rule can consume (rustfmt, clippy) bind; the rest
// are recognized and targeted uniformly for future language owners.
type nativeTool struct {
	id   string
	file string
	kind string
	rust bool
}

var nativeTools = []nativeTool{
	{id: "buildifier", file: ".buildifier.json", kind: "buildifier_config"},
	{id: "taplo", file: "taplo.toml", kind: "taplo_config"},
	{id: "vale", file: ".vale.ini", kind: "vale_config"},
	{id: "rustfmt", file: "rustfmt.toml", kind: "rustfmt_config", rust: true},
	{id: "clippy", file: "clippy.toml", kind: "clippy_config", rust: true},
}

// nativeToolByID resolves a tool id to its table row.
func nativeToolByID(id string) (nativeTool, bool) {
	for _, tool := range nativeTools {
		if tool.id == id {
			return tool, true
		}
	}
	return nativeTool{}, false
}

// nativeToolByKind resolves a config rule kind to its table row.
func nativeToolByKind(kind string) (nativeTool, bool) {
	for _, tool := range nativeTools {
		if tool.kind == kind {
			return tool, true
		}
	}
	return nativeTool{}, false
}

// nativeConfigName is the deterministic generated target name for a tool.
func nativeConfigName(tool nativeTool) string {
	return tool.id + "_config"
}

// defaultNativeTools selects every managed tool in table order.
func defaultNativeTools() []string {
	ids := make([]string, 0, len(nativeTools))
	for _, tool := range nativeTools {
		ids = append(ids, tool.id)
	}
	return ids
}

// nativeToolIDs renders the valid directive values for diagnostics.
func nativeToolIDs() string {
	return strings.Join(defaultNativeTools(), " ")
}

// parseNativeToolsDirective validates one dx_native_tools value into the
// enabled tool set, preserving directive order.
func parseNativeToolsDirective(value string) ([]string, error) {
	fields := strings.Fields(value)
	if len(fields) == 0 {
		return nil, fmt.Errorf("rust: malformed # gazelle:%s: want space-separated tool ids (%s)", nativeToolsDirective, nativeToolIDs())
	}
	seen := make(map[string]bool, len(fields))
	tools := make([]string, 0, len(fields))
	for _, field := range fields {
		tool, ok := nativeToolByID(field)
		if !ok {
			return nil, fmt.Errorf("rust: unknown native tool %q in # gazelle:%s (want one of: %s)", field, nativeToolsDirective, nativeToolIDs())
		}
		if !seen[tool.id] {
			seen[tool.id] = true
			tools = append(tools, tool.id)
		}
	}
	return tools, nil
}

// selectedNativeTools reports the managed tool set for a directory,
// defaulting to every tool when configuration never ran (unit tests with
// bare configs) or no directive constrained it.
func selectedNativeTools(c *config.Config) []string {
	if c != nil {
		if raw, ok := c.Exts[languageName]; ok {
			if rc, ok := raw.(*rustConfig); ok && rc != nil && rc.tools != nil {
				return rc.tools
			}
		}
	}
	return defaultNativeTools()
}

// nativeConfigKinds declares the managed config rule kinds: matched by
// src identity, src required for buildability. src and data are mergeable
// so removal stubs clear them and the emptied rule deletes: the planner
// only ever emits a config rule when no same-tool target exists, so a
// mergeable src can never clobber a hand-authored one.
func nativeConfigKinds() map[string]rule.KindInfo {
	kinds := make(map[string]rule.KindInfo, len(nativeTools))
	for _, tool := range nativeTools {
		kinds[tool.kind] = rule.KindInfo{
			MatchAttrs:    []string{"src"},
			NonEmptyAttrs: map[string]bool{"src": true},
			MergeableAttrs: map[string]bool{
				"data": true,
				"src":  true,
			},
		}
	}
	return kinds
}

// isNativeConfigKind reports whether a kind is a managed config target.
func isNativeConfigKind(kind string) bool {
	_, ok := nativeToolByKind(kind)
	return ok
}

// nativeConfigLoads exposes the typed config constructors for generated
// targets. Gazelle materializes only the symbols generation uses.
func nativeConfigLoads(rulesRepo string) rule.LoadInfo {
	symbols := make([]string, 0, len(nativeTools))
	for _, tool := range nativeTools {
		symbols = append(symbols, tool.kind)
	}
	return rule.LoadInfo{
		Name:    "@" + rulesRepo + "//quality:native_config.bzl",
		Symbols: symbols,
	}
}

// nativePlan is the config-target computation for one directory: rules to
// generate, owned rules to delete, and the resolution map hint binding
// consumes.
type nativePlan struct {
	gen       []*rule.Rule
	imports   []any
	empty     []*rule.Rule
	resolved  map[string]string
	generated map[string]bool
	stubbed   map[string]bool
	existing  map[string]*rule.Rule
}

// planNativeConfig recognizes selected tool files, resolves each tool to
// its single config target, and stages generation and removal. More than
// one same-tool config target fails closed: hint binding must stay
// unambiguous.
func planNativeConfig(c *config.Config, args language.GenerateArgs) (*nativePlan, error) {
	plan := &nativePlan{
		resolved:  make(map[string]string, len(nativeTools)),
		generated: make(map[string]bool),
		stubbed:   make(map[string]bool),
		existing:  make(map[string]*rule.Rule),
	}
	if args.File != nil {
		for _, existing := range args.File.Rules {
			plan.existing[existing.Name()] = existing
		}
	}
	present := make(map[string]bool, len(args.RegularFiles))
	for _, name := range args.RegularFiles {
		present[name] = true
	}
	selected := make(map[string]bool, len(nativeTools))
	for _, id := range selectedNativeTools(c) {
		selected[id] = true
	}
	for _, tool := range nativeTools {
		var owned []string
		for name, existing := range plan.existing {
			if existing.Kind() == tool.kind {
				owned = append(owned, name)
			}
		}
		if len(owned) > 1 {
			sort.Strings(owned)
			return nil, fmt.Errorf("rust: //%s: tool %q has %d config targets (%s); keep one so aspect_hints stays unambiguous",
				args.Rel, tool.id, len(owned), strings.Join(owned, ", "))
		}
		name := nativeConfigName(tool)
		if len(owned) == 1 {
			// A hand-authored target (any name, any src) always wins:
			// never regenerated, never stubbed, always resolvable.
			plan.resolved[tool.id] = owned[0]
			continue
		}
		if !selected[tool.id] || !present[tool.file] {
			continue
		}
		r := rule.NewRule(tool.kind, name)
		r.SetAttr("src", tool.file)
		if data := valeStylesData(args.Config.RepoRoot, args.Dir, args.Rel, tool); len(data) > 0 {
			r.SetAttr("data", data)
		}
		r.SetAttr("visibility", []string{nativeVisibility(args.Rel)})
		plan.gen = append(plan.gen, r)
		plan.imports = append(plan.imports, targetImports{})
		plan.generated[name] = true
		plan.resolved[tool.id] = name
	}
	for _, tool := range nativeTools {
		name := nativeConfigName(tool)
		existing, ok := plan.existing[name]
		if !ok || existing.Kind() != tool.kind || plan.generated[name] {
			continue
		}
		// Only a rule the generator could have emitted (default name
		// plus the recognized src) is ever stubbed: hand-customized
		// targets survive file removal and deselection untouched.
		if existing.AttrString("src") != tool.file {
			continue
		}
		if selected[tool.id] && present[tool.file] {
			continue
		}
		plan.empty = append(plan.empty, rule.NewRule(tool.kind, name))
		plan.stubbed[name] = true
		// A stubbed target no longer exists after the merge, so it
		// must not resolve for hint binding either.
		if plan.resolved[tool.id] == name {
			delete(plan.resolved, tool.id)
		}
	}
	return plan, nil
}

// nativeVisibility scopes a generated config target to its owning package
// tree: consumers resolve hints package-locally, never repo-wide.
func nativeVisibility(rel string) string {
	if rel == "" {
		return "//:__subpackages__"
	}
	return "//" + rel + ":__subpackages__"
}

// applyNativeHints binds one generated Rust rule to every selected tool
// whose config target exists in its package. The emitted list is the full
// merged outcome: surviving entries keep their relative position, stale
// managed entries drop, and resolved additions append in canonical-label
// order. An empty outcome stays absent so the merger deletes a fully
// stale attribute instead of rendering an empty list.
func applyNativeHints(rel string, r *rule.Rule, plan *nativePlan) {
	if _, owned := rustKinds[r.Kind()]; !owned || isNativeConfigKind(r.Kind()) {
		return
	}
	existing := plan.existing[r.Name()]
	if existing != nil && existing.Kind() != r.Kind() {
		existing = nil
	}
	var merged []string
	if existing != nil {
		for _, entry := range existing.AttrStrings("aspect_hints") {
			name, managed := managedHintName(rel, entry)
			if managed && !plan.targetPresent(name) {
				continue
			}
			merged = append(merged, entry)
		}
	}
	present := make(map[string]bool, len(merged))
	for _, entry := range merged {
		present[hintCanonicalName(rel, entry)] = true
	}
	var additions []string
	for _, tool := range nativeTools {
		if !tool.rust {
			continue
		}
		name, ok := plan.resolved[tool.id]
		if !ok || present[name] {
			continue
		}
		additions = append(additions, ":"+name)
	}
	sort.Strings(additions)
	merged = append(merged, additions...)
	if len(merged) == 0 {
		return
	}
	r.SetAttr("aspect_hints", merged)
}

// targetPresent reports whether a hint label still resolves inside the
// package: generated this run, or existing and not scheduled for removal.
func (plan *nativePlan) targetPresent(name string) bool {
	if plan.generated[name] {
		return true
	}
	if _, ok := plan.existing[name]; !ok || plan.stubbed[name] {
		return false
	}
	return true
}

// managedHintName resolves an aspect_hints entry to its package-local
// target name when the entry uses a managed default label form.
func managedHintName(rel, entry string) (string, bool) {
	for _, tool := range nativeTools {
		if !tool.rust {
			continue
		}
		name := nativeConfigName(tool)
		if entry == ":"+name {
			return name, true
		}
		if rel == "" {
			if entry == "//:"+name {
				return name, true
			}
		} else if entry == "//"+rel+":"+name {
			return name, true
		}
	}
	return "", false
}

// hintCanonicalName normalizes an aspect_hints entry to its package-local
// target name for deduplication; foreign labels pass through unchanged so
// they never collide with local additions.
func hintCanonicalName(rel, entry string) string {
	if name, managed := managedHintName(rel, entry); managed {
		return name
	}
	return entry
}

// valeStylesData closes over a Vale styles directory: the StylesPath entry
// of the recognized INI (default styles) walks recursively into sorted
// package-relative data labels. Absent files, absent directories, and
// escaping paths all yield no data instead of failing: a self-contained
// INI without styles is an ordinary config.
func valeStylesData(repoRoot, dir, rel string, tool nativeTool) []string {
	if tool.id != "vale" {
		return nil
	}
	content, err := os.ReadFile(filepath.Join(dir, tool.file))
	if err != nil {
		return nil
	}
	styles := "styles"
	for _, line := range strings.Split(string(content), "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") || strings.HasPrefix(trimmed, ";") || strings.HasPrefix(trimmed, "[") {
			continue
		}
		key, value, ok := strings.Cut(trimmed, "=")
		if !ok || !strings.EqualFold(strings.TrimSpace(key), "StylesPath") {
			continue
		}
		styles = strings.Trim(strings.TrimSpace(value), `"'`)
	}
	if styles == "" || path.IsAbs(styles) || filepath.IsAbs(styles) {
		return nil
	}
	var data []string
	root := filepath.Join(dir, filepath.FromSlash(styles))
	_ = filepath.WalkDir(root, func(name string, entry os.DirEntry, err error) error {
		if err != nil || entry.IsDir() {
			return nil
		}
		local, relErr := filepath.Rel(dir, name)
		if relErr != nil {
			return nil // LCOV_EXCL_LINE - reason: WalkDir only yields paths beneath its root and both inputs are already host-native paths, so filepath.Rel cannot fail on the seed host.
		} // LCOV_EXCL_LINE - reason: closing brace belongs only to the unreachable filepath.Rel error guard above.
		slash := filepath.ToSlash(local)
		if slash == ".." || strings.HasPrefix(slash, "../") {
			return nil
		}
		data = append(data, path.Join(rel, slash))
		return nil
	})
	sort.Strings(data)
	return data
}
