// Intended-manifest recorder (WP1, dispatch).
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
// nil, nil when recording is disabled, so direct Gazelle invocations
// behave exactly as before. Misconfiguration is a returned error, never a
// panic: the caller records it through the language error accumulator and
// the run fails closed before BUILD emission.
func loadManifestRecorder() (*manifestRecorder, error) {
	outPath := os.Getenv(envIntendedManifest)
	if outPath == "" {
		return nil, nil
	}
	mode := os.Getenv(envGenerateMode)
	if mode == "" {
		mode = "default"
	}
	if mode != "check" && mode != "default" {
		return nil, fmt.Errorf("rust: %s must be \"check\" or \"default\", got %q", envGenerateMode, mode)
	}
	scopes := []scopeElement{{Element: "//...", Dirs: []string{""}}}
	if raw := os.Getenv(envGenerateScope); raw != "" {
		if err := json.Unmarshal([]byte(raw), &scopes); err != nil {
			return nil, fmt.Errorf("rust: malformed %s: %v", envGenerateScope, err)
		}
		if len(scopes) == 0 {
			return nil, fmt.Errorf("rust: %s must list at least one scope element", envGenerateScope)
		}
	}
	return &manifestRecorder{outPath: outPath, mode: mode, scopes: scopes}, nil
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
// is exactly when the framework calls AfterResolvingDeps. Scope and
// transport failures are returned for the language error accumulator,
// never panics: a typo in the wrapper environment must fail the run with
// an actionable message, not a Go stack trace.
func (r *manifestRecorder) emit(ignores []*ignoreEntry) error {
	manifest := intendedManifest{
		SchemaMajor: intendedManifestSchemaMajor,
		SchemaMinor: intendedManifestSchemaMinor,
		Mode:        r.mode,
		// The Rust finalizer expects sequences, but Go marshals nil slices
		// as null. Initialize empty so clean runs emit [] not null.
		Scopes:         []intendedScope{},
		Files:          []intendedFile{},
		IgnoredImports: []intendedIgnoredImport{},
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
			return fmt.Errorf("rust: package %q matches no %s scope element", rec.rel, envGenerateScope)
		}
		file, changed, err := r.witness(rec)
		if err != nil {
			return err
		}
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
			return fmt.Errorf("rust: ignored import %q matches no %s scope element", ignore.value, envGenerateScope)
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
		// LCOV_EXCL_START - reason: unreachable encode, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
		if a.Language != b.Language {
			return a.Language < b.Language
		}
		// LCOV_EXCL_STOP - reason: end unreachable encode, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
		return a.Import < b.Import
	})
	data, err := json.Marshal(manifest)
	if err != nil {
		return fmt.Errorf("rust: cannot encode intended manifest: %v", err) // LCOV_EXCL_LINE - reason: unreachable encode, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
	}
	if err := os.WriteFile(r.outPath, data, 0o600); err != nil {
		return fmt.Errorf("rust: cannot write intended manifest: %v", err)
	}
	return nil
}

// witness replays the framework's own load fixing and formatting for one
// visited package and diffs against the original bytes. It reports whether
// the package changed.
func (r *manifestRecorder) witness(rec packageRecord) (intendedFile, bool, error) {
	var file intendedFile
	if rec.file == nil {
		if len(rec.gen) == 0 {
			// Mirrors the framework: no file is created when nothing was
			// generated for a directory without a BUILD file.
			return file, false, nil
		}
		built := rule.EmptyFile(filepath.Join(rec.dir, rec.cfg.DefaultBuildFileName()), rec.rel)
		for _, g := range rec.gen {
			g.Insert(built)
		}
		loads, err := r.knownLoads(rec)
		if err != nil {
			return file, false, err
		}
		merger.FixLoads(built, loads)
		file.Path = manifestPath(rec.rel, rec.cfg.DefaultBuildFileName())
		file.CreateContent = built.Format()
		return file, true, nil
	}
	original := rec.file.Content
	loads, err := r.knownLoads(rec)
	if err != nil {
		return file, false, err
	}
	merger.FixLoads(rec.file, loads)
	intended := rec.file.Format()
	if bytes.Equal(original, intended) {
		return file, false, nil
	}
	file.Path = manifestPath(rec.rel, filepath.Base(rec.file.Path))
	file.OriginalContent = original
	file.Edits = diffLines(original, intended)
	if len(file.Edits) == 0 {
		return file, false, fmt.Errorf("rust: changed package %q produced no edits", rec.rel) // LCOV_EXCL_LINE - reason: unreachable encode, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
	}
	return file, true, nil
}

// knownLoads replays the framework's load collection for the recorder's
// language: ApparentLoads with the run's own module mapping, plus the
// kind-mapping adjustment for the package's pre-replacement kinds. With no
// map_kind matches this is exactly the framework's load list, and
// merger.FixLoads converges, so the replayed bytes equal the emitted ones.
// A cyclic kind mapping is a configuration error, returned for the language
// error accumulator instead of panicking.
func (r *manifestRecorder) knownLoads(rec packageRecord) ([]rule.LoadInfo, error) {
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
// resolution over the run's KindMap. A cyclic mapping is a configuration
// error, returned instead of panicking.
func applyKindMappings(rec packageRecord, loads []rule.LoadInfo) ([]rule.LoadInfo, error) {
	var mapped []config.MappedKind
	for _, kind := range append(append([]string{}, rec.genKinds...), rec.oldKinds...) {
		repl, err := replacementKind(rec.cfg.KindMap, kind)
		if err != nil {
			return nil, err
		}
		if repl != nil {
			mapped = append(mapped, *repl)
		}
	}
	if len(mapped) == 0 {
		return loads, nil
	}
	merged := make([]rule.LoadInfo, len(loads))
	copy(merged, loads)
	for _, repl := range mapped {
		merged = appendOrMergeKindMapping(merged, repl)
	}
	return merged, nil
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
// A cyclic mapping is a configuration error: the recorder detects it here
// and returns an error so the run fails closed with an actionable message
// instead of a Go stack trace.
func replacementKind(kindMap map[string]config.MappedKind, kind string) (*config.MappedKind, error) {
	var mapped *config.MappedKind
	seen := make(map[string]struct{})
	for {
		replacement, ok := kindMap[kind]
		if !ok {
			return mapped, nil
		}
		if _, dup := seen[replacement.KindName]; dup {
			return nil, fmt.Errorf("rust: kind map loop at %q", replacement.KindName)
		}
		seen[replacement.KindName] = struct{}{}
		current := replacement
		mapped = &current
		if kind == replacement.KindName {
			return mapped, nil
		}
		kind = replacement.KindName
	}
}

// nonNilBytes normalizes nil to empty so JSON marshals "" not null.
func nonNilBytes(b []byte) []byte {
	if b == nil {
		return []byte{}
	}
	return b
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
			Start: oldOffsets[code.I1],
			End:   oldOffsets[code.I2],
			// bytes.Join returns nil for pure deletions; Go marshals nil
			// []byte as null, but the Rust finalizer expects a base64
			// string. Normalize to empty so deletions emit "".
			Replacement: nonNilBytes(bytes.Join(newLines[code.J1:code.J2], nil)),
		}
		// LCOV_EXCL_START - reason: unreachable encode, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
		if bytes.Equal(original[edit.Start:edit.End], edit.Replacement) {
			continue
		}
		// LCOV_EXCL_STOP - reason: end unreachable encode, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
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
