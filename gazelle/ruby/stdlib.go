// Stdlib lists the Ruby require roots treated as standard library by the
// Ruby Gazelle extension (ADR 0032). Generation treats these imports as
// interpreter-provided without an edge, a manifest, or a lockfile. The set
// covers the common interpreter-bundled libraries; it is revisited when
// ADR 0032 freezes the toolchain baseline.
package ruby

// stdlibRoots are the require identities treated as interpreter-provided.
var stdlibRoots = []string{
	"json",
	"yaml",
	"net/http",
	"uri",
	"fileutils",
	"pathname",
	"set",
	"optparse",
	"logger",
	"stringio",
	"strscan",
	"date",
	"time",
	"openssl",
	"digest",
	"base64",
	"cgi",
	"erb",
	"psych",
}

// IsStdLib reports whether a require identity is interpreter-provided.
func IsStdLib(spec string) bool {
	trimmed := spec
	for _, clean := range stdlibRoots {
		if trimmed == clean || len(trimmed) > len(clean) && trimmed[:len(clean)] == clean && (trimmed[len(clean)] == '/' || trimmed[len(clean)] == '.') {
			return true
		}
	}
	return false
}
