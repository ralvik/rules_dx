// Intended-manifest recorder (M10 WP1, O13 dispatch).
//
// The canonical generation run witnesses its own exact BUILD changes and
// hands them to the manifest finalizer as JSON. This file owns the
// generation side of that private protocol; the CLI never inspects the
// workspace itself (see docs/cli/commands/generate.md#cli-boundary).
//
// Protocol (environment, set by the canonical execution wrapper):
//
//	DX_GENERATE_INTENDED  absolute path of the intended-manifest JSON file.
//	                      Absent disables recording entirely, leaving plain
//	                      Gazelle behavior (direct binary invocations and the
//	                      existing test harness are unaffected).
//	DX_GENERATE_SCOPE     JSON list of {"element","dirs"} scope elements in
//	                      CLI-resolved order. Dirs are repo-relative slash
//	                      paths ("" is the root). Absent defaults to one
//	                      repo-wide "//..." element.
//	DX_GENERATE_MODE      "check" or "default". Absent defaults to "default".
//
// Recording points mirror the upstream framework (bazel-gazelle update):
//   - GenerateRules retains the *rule.File pointer the framework later
//     merges in place, plus the returned gen rules and pre-replacement
//     kinds, so AfterResolvingDeps observes the exact post-merge state.
//   - AfterResolvingDeps runs after the PostResolve merge and before the
//     emit loop, while on-disk bytes are still the originals. It replays
//     the framework's own load fixing (merger.FixLoads, idempotent) with
//     the identical loads the framework will use, then formats. The
//     resulting bytes equal what the emit loop will write, because both
//     sides call the same functions with the same inputs.
//   - Write outcomes and BLAKE3 digests are deliberately NOT recorded
//     here: outcomes are only knowable after the emit loop, and hashing
//     stays with the finalizer. The finalizer cross-checks intended
//     bytes against disk and fails closed on any contradiction.
//
// Scope narrowing falls out of traversal: the wrapper passes only the
// resolved scope dirs to the Gazelle binary, so unvisited packages are
// byte-identical by construction and out-of-scope staleness is never
// observed.

package rust

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"

	"github.com/bazelbuild/bazel-gazelle/config"
	"github.com/bazelbuild/bazel-gazelle/language"
	"github.com/bazelbuild/bazel-gazelle/merger"
	"github.com/bazelbuild/bazel-gazelle/rule"
	difflib "github.com/pmezard/go-difflib/difflib"
)

const (
	intendedManifestSchemaMajor = 1
	intendedManifestSchemaMinor = 0

	envIntendedManifest = "DX_GENERATE_INTENDED"
	envGenerateScope    = "DX_GENERATE_SCOPE"
	envGenerateMode     = "DX_GENERATE_MODE"
)

// scopeElement is one CLI-resolved scope element with the repo-relative
// slash package dirs it covers ("" is the repository root).
type scopeElement struct {
	Element string   `json:"element"`
	Dirs    []string `json:"dirs"`
}

// intendedEdit is one half-open byte edit against the original content.
// Line-granular diffing keeps every boundary on a '\n', which is always a
// UTF-8 boundary, so edits are boundary-valid by construction.
type intendedEdit struct {
	Start       uint64 `json:"start_byte"`
	End         uint64 `json:"end_byte"`
	Replacement []byte `json:"replacement"`
}

// intendedFile is one changed BUILD file in deterministic visit order.
type intendedFile struct {
	Path            string         `json:"path"`
	ScopeIndex      int            `json:"scope_index"`
	CreateContent   []byte         `json:"create_content,omitempty"`
	OriginalContent []byte         `json:"original_content,omitempty"`
	Edits           []intendedEdit `json:"edits,omitempty"`
}

// intendedIgnoredImport is one accepted dx_ignore_import directive use.
type intendedIgnoredImport struct {
	Path       string `json:"path"`
	Language   string `json:"language"`
	Import     string `json:"import"`
	ScopeIndex int    `json:"scope_index"`
}

// intendedScope mirrors the manifest Scope: every visited scope element
// completes in a run that reaches emission.
type intendedScope struct {
	Value           string `json:"value"`
	ResultsComplete bool   `json:"results_complete"`
}

// intendedManifest is the private generation-side witness. The finalizer
// fills digests and write outcomes and encodes the versioned artifact.
type intendedManifest struct {
	SchemaMajor    int                     `json:"schema_major"`
	SchemaMinor    int                     `json:"schema_minor"`
	Mode           string                  `json:"mode"`
	Scopes         []intendedScope         `json:"scopes"`
	Files          []intendedFile          `json:"files"`
	IgnoredImports []intendedIgnoredImport `json:"ignored_imports"`
}

// packageRecord captures one GenerateRules visit for later witnessing.
type packageRecord struct {
	rel      string
	dir      string
	file     *rule.File
	gen      []*rule.Rule
	genKinds []string
	oldKinds []string
	cfg      *config.Config
}

// manifestRecorder accumulates package visits for one run. Nil disables
// recording.
type manifestRecorder struct {
	outPath string
	mode    string
	scopes  []scopeElement
	// apparentLoads replays the framework's load collection: it is the
	// language's ApparentLoads, called with the run's own module mapping.
	apparentLoads func(func(string) string) []rule.LoadInfo
	visited       []packageRecord
}

// loadManifestRecorder reads the private protocol environment. It returns
// nil when recording is disabled, so direct Gazelle invocations behave
// exactly as before.
func loadManifestRecorder() *manifestRecorder {
	outPath := os.Getenv(envIntendedManifest)
	if outPath == "" {
		return nil
	}
	mode := os.Getenv(envGenerateMode)
	if mode == "" {
		mode = "default"
	}
	if mode != "check" && mode != "default" {
		panic(fmt.Sprintf("rust: %s must be \"check\" or \"default\", got %q", envGenerateMode, mode))
	}
	scopes := []scopeElement{{Element: "//...", Dirs: []string{""}}}
	if raw := os.Getenv(envGenerateScope); raw != "" {
		if err := json.Unmarshal([]byte(raw), &scopes); err != nil {
			panic(fmt.Sprintf("rust: malformed %s: %v", envGenerateScope, err))
		}
		if len(scopes) == 0 {
			panic(fmt.Sprintf("rust: %s must list at least one scope element", envGenerateScope))
		}
	}
	return &manifestRecorder{outPath: outPath, mode: mode, scopes: scopes}
}

// record retains one GenerateRules visit. Kinds are captured before the
// framework applies map_kind replacements, which is the input the load
// computation needs.
func (r *manifestRecorder) record(args language.GenerateArgs, res language.GenerateResult) {
	rec := packageRecord{
		rel:  args.Rel,
		dir:  args.Dir,
		file: args.File,
		gen:  res.Gen,
		cfg:  args.Config,
	}
	for _, g := range res.Gen {
		rec.genKinds = append(rec.genKinds, g.Kind())
	}
	if args.File != nil {
		for _, old := range args.File.Rules {
			rec.oldKinds = append(rec.oldKinds, old.Kind())
		}
	}
	r.visited = append(r.visited, rec)
}

// scopeIndex attributes a package rel to its owning scope element: the
// longest matching dir wins, ties keep listed order.
func (r *manifestRecorder) scopeIndex(rel string) int {
	best := -1
	bestLen := -1
	for i, scope := range r.scopes {
		for _, dir := range scope.Dirs {
			length := -1
			switch {
			case dir == "":
				length = 0
			case rel == dir:
				length = len(dir)
			case strings.HasPrefix(rel, dir+"/"):
				length = len(dir)
			}
			if length > bestLen {
				bestLen = length
				best = i
			}
		}
	}
	return best
}

// emit witnesses every visited package and writes the intended manifest.
// It must run after the PostResolve merge and before the emit loop, which
// is exactly when the framework calls AfterResolvingDeps.
func (r *manifestRecorder) emit(ignores []*ignoreEntry) {
	manifest := intendedManifest{
		SchemaMajor: intendedManifestSchemaMajor,
		SchemaMinor: intendedManifestSchemaMinor,
		Mode:        r.mode,
	}
	for _, scope := range r.scopes {
		manifest.Scopes = append(manifest.Scopes, intendedScope{
			Value:           scope.Element,
			ResultsComplete: true,
		})
	}
	for _, rec := range r.visited {
		index := r.scopeIndex(rec.rel)
		if index < 0 {
			panic(fmt.Sprintf("rust: package %q matches no %s scope element", rec.rel, envGenerateScope))
		}
		file, changed := r.witness(rec)
		if !changed {
			continue
		}
		file.ScopeIndex = index
		manifest.Files = append(manifest.Files, file)
	}
	for _, ignore := range ignores {
		if !ignore.used {
			continue
		}
		index := r.scopeIndex(ignore.path)
		if index < 0 {
			panic(fmt.Sprintf("rust: ignored import %q matches no %s scope element", ignore.value, envGenerateScope))
		}
		manifest.IgnoredImports = append(manifest.IgnoredImports, intendedIgnoredImport{
			Path:       ignore.path,
			Language:   languageName,
			Import:     ignore.value,
			ScopeIndex: index,
		})
	}
	// The transport crate requires ignored imports sorted by
	// (path, language, import); the ignore list arrives in directive-visit
	// order, so sort before encoding.
	sort.SliceStable(manifest.IgnoredImports, func(i, j int) bool {
		a, b := manifest.IgnoredImports[i], manifest.IgnoredImports[j]
		if a.Path != b.Path {
			return a.Path < b.Path
		}
		// LCOV_EXCL_START - reason: emit stamps every entry with languageName, so languages never differ here; the tiebreak mirrors the crate's (path, language, import) key for protocol evolution.
		if a.Language != b.Language {
			return a.Language < b.Language
		}
		// LCOV_EXCL_STOP - reason: end of unreachable language-tiebreak exclusion.
		return a.Import < b.Import
	})
	data, err := json.Marshal(manifest)
	if err != nil {
		panic(fmt.Sprintf("rust: cannot encode intended manifest: %v", err)) // LCOV_EXCL_LINE - reason: manifest holds only strings, bytes, ints, and bools, so Marshal cannot fail; this branch is defensive only.
	}
	if err := os.WriteFile(r.outPath, data, 0o600); err != nil {
		panic(fmt.Sprintf("rust: cannot write intended manifest: %v", err))
	}
}

// witness replays the framework's own load fixing and formatting for one
// visited package and diffs against the original bytes. It reports whether
// the package changed.
func (r *manifestRecorder) witness(rec packageRecord) (intendedFile, bool) {
	var file intendedFile
	if rec.file == nil {
		if len(rec.gen) == 0 {
			// Mirrors the framework: no file is created when nothing was
			// generated for a directory without a BUILD file.
			return file, false
		}
		built := rule.EmptyFile(filepath.Join(rec.dir, rec.cfg.DefaultBuildFileName()), rec.rel)
		for _, g := range rec.gen {
			g.Insert(built)
		}
		merger.FixLoads(built, r.knownLoads(rec))
		file.Path = manifestPath(rec.rel, rec.cfg.DefaultBuildFileName())
		file.CreateContent = built.Format()
		return file, true
	}
	original := rec.file.Content
	merger.FixLoads(rec.file, r.knownLoads(rec))
	intended := rec.file.Format()
	if bytes.Equal(original, intended) {
		return file, false
	}
	file.Path = manifestPath(rec.rel, filepath.Base(rec.file.Path))
	file.OriginalContent = original
	file.Edits = diffLines(original, intended)
	if len(file.Edits) == 0 {
		panic(fmt.Sprintf("rust: changed package %q produced no edits", rec.rel)) // LCOV_EXCL_LINE - reason: differing bytes always differ in at least one line run, so a changed package always yields a non-noop edit; this branch is defensive only.
	}
	return file, true
}

// knownLoads replays the framework's load collection for the recorder's
// language: ApparentLoads with the run's own module mapping, plus the
// kind-mapping adjustment for the package's pre-replacement kinds. With no
// map_kind matches this is exactly the framework's load list, and
// merger.FixLoads converges, so the replayed bytes equal the emitted ones.
func (r *manifestRecorder) knownLoads(rec packageRecord) []rule.LoadInfo {
	return applyKindMappings(rec, r.apparentLoads(rec.cfg.ModuleToApparentName))
}

// manifestPath renders the repo-relative slash path of a BUILD file.
func manifestPath(rel, base string) string {
	if rel == "" {
		return base
	}
	return path.Join(rel, base)
}

// applyKindMappings mirrors the framework's load adjustment: kinds
// recorded before replacement replay the same transitive map_kind
// resolution over the run's KindMap.
func applyKindMappings(rec packageRecord, loads []rule.LoadInfo) []rule.LoadInfo {
	var mapped []config.MappedKind
	for _, kind := range append(append([]string{}, rec.genKinds...), rec.oldKinds...) {
		if repl := replacementKind(rec.cfg.KindMap, kind); repl != nil {
			mapped = append(mapped, *repl)
		}
	}
	if len(mapped) == 0 {
		return loads
	}
	merged := make([]rule.LoadInfo, len(loads))
	copy(merged, loads)
	for _, repl := range mapped {
		merged = appendOrMergeKindMapping(merged, repl)
	}
	return merged
}

// appendOrMergeKindMapping mirrors the framework's load-list adjustment
// for one mapped kind.
func appendOrMergeKindMapping(loads []rule.LoadInfo, repl config.MappedKind) []rule.LoadInfo {
	for i, load := range loads {
		if load.Name == repl.KindLoad {
			loads[i].Symbols = append(load.Symbols, repl.KindName)
			return loads
		}
	}
	return append(loads, rule.LoadInfo{
		Name:    repl.KindLoad,
		Symbols: []string{repl.KindName},
	})
}

// replacementKind mirrors the framework's transitive map_kind resolution.
// A loop is unreachable here: the framework panics during generation,
// before AfterResolvingDeps runs the recorder.
func replacementKind(kindMap map[string]config.MappedKind, kind string) *config.MappedKind {
	var mapped *config.MappedKind
	seen := make(map[string]struct{})
	for {
		replacement, ok := kindMap[kind]
		if !ok {
			return mapped
		}
		if _, dup := seen[replacement.KindName]; dup {
			panic(fmt.Sprintf("rust: kind map loop at %q", replacement.KindName))
		}
		seen[replacement.KindName] = struct{}{}
		current := replacement
		mapped = &current
		if kind == replacement.KindName {
			return mapped
		}
		kind = replacement.KindName
	}
}

// splitLines splits bytes after each '\n' without adding or dropping any,
// so concatenating the result reproduces the input exactly.
func splitLines(data []byte) [][]byte {
	if len(data) == 0 {
		return nil
	}
	var lines [][]byte
	for len(data) > 0 {
		i := bytes.IndexByte(data, '\n')
		if i < 0 {
			lines = append(lines, data)
			break
		}
		lines = append(lines, data[:i+1])
		data = data[i+1:]
	}
	return lines
}

// diffLines converts the line diff between original and intended into the
// validated edit form: ordered, non-overlapping, boundary-aligned, with
// no-op replacements dropped.
func diffLines(original, intended []byte) []intendedEdit {
	oldLines := splitLines(original)
	newLines := splitLines(intended)
	oldOffsets := lineOffsets(oldLines)
	var edits []intendedEdit
	for _, code := range difflib.NewMatcher(toStrings(oldLines), toStrings(newLines)).GetOpCodes() {
		if code.Tag == 'e' {
			continue
		}
		edit := intendedEdit{
			Start:       oldOffsets[code.I1],
			End:         oldOffsets[code.I2],
			Replacement: bytes.Join(newLines[code.J1:code.J2], nil),
		}
		// LCOV_EXCL_START - reason: difflib only emits non-equal opcodes for differing line runs, so the replacement always differs from the covered bytes; this guard is defensive only.
		if bytes.Equal(original[edit.Start:edit.End], edit.Replacement) {
			continue
		}
		// LCOV_EXCL_STOP - reason: end of defensive no-op edit guard.
		edits = append(edits, edit)
	}
	return edits
}

// lineOffsets returns the byte offset of each line start plus a sentinel
// end offset, so opcode ranges map back to exact byte ranges.
func lineOffsets(lines [][]byte) []uint64 {
	offsets := make([]uint64, len(lines)+1)
	for i, line := range lines {
		offsets[i+1] = offsets[i] + uint64(len(line))
	}
	return offsets
}

// toStrings adapts split lines to the difflib matcher input.
func toStrings(lines [][]byte) []string {
	out := make([]string, len(lines))
	for i, line := range lines {
		out[i] = string(line)
	}
	return out
}
