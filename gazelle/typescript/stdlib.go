// Standard-library identities for the TypeScript Gazelle extension.
//
// TypeScript executes on Node, so the recognized builtins are the Node
// builtins (including `node:`-prefixed and `test`/`node:test`). They
// resolve with no edge: an import of a builtin never becomes a Bazel
// dependency. Everything else resolves strictly to a local one-source
// target or fails with an exact-mapping remediation.
package typescript

import "strings"

// nodeBuiltins is the exact recognized Node builtin set for the M16 slice,
// mirroring the JavaScript extension (Node release backing the pinned
// toolchain). Subpaths (`fs/promises`) match exactly, never by prefix:
// deeper unknown paths (`fs/promises/extra`) fail closed via strict
// resolution unless mapped or ignored.
var nodeBuiltins = map[string]bool{
	"assert":              true,
	"async_hooks":         true,
	"buffer":              true,
	"child_process":       true,
	"cluster":             true,
	"console":             true,
	"constants":           true,
	"crypto":              true,
	"dgram":               true,
	"diagnostics_channel": true,
	"dns":                 true,
	"domain":              true,
	"events":              true,
	"fs":                  true,
	"fs/promises":         true,
	"http":                true,
	"http2":               true,
	"https":               true,
	"inspector":           true,
	"inspector/promises":  true,
	"module":              true,
	"net":                 true,
	"os":                  true,
	"path":                true,
	"path/posix":          true,
	"path/win32":          true,
	"perf_hooks":          true,
	"process":             true,
	"punycode":            true,
	"querystring":         true,
	"readline":            true,
	"readline/promises":   true,
	"repl":                true,
	"sea":                 true,
	"sqlite":              true,
	"stream":              true,
	"stream/consumers":    true,
	"stream/promises":     true,
	"stream/web":          true,
	"string_decoder":      true,
	"sys":                 true,
	"test":                true,
	"test/reporters":      true,
	"timers":              true,
	"timers/promises":     true,
	"tls":                 true,
	"trace_events":        true,
	"tty":                 true,
	"url":                 true,
	"util":                true,
	"util/types":          true,
	"v8":                  true,
	"vm":                  true,
	"wasi":                true,
	"worker_threads":      true,
	"zlib":                true,
}

// IsStdLib reports whether a normalized import root is a recognized Node
// builtin. The `node:` prefix is stripped before lookup; subpaths resolve
// by exact match only (`fs/promises` matches, `fs/promises/extra` does not).
func IsStdLib(root string) bool {
	name := strings.TrimPrefix(root, "node:")
	if name == "" {
		return false
	}
	return nodeBuiltins[name]
}
