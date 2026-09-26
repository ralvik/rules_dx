package ruby

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

func IsStdLib(spec string) bool {
	trimmed := spec
	for _, clean := range stdlibRoots {
		if trimmed == clean || len(trimmed) > len(clean) && trimmed[:len(clean)] == clean && (trimmed[len(clean)] == '/' || trimmed[len(clean)] == '.') {
			return true
		}
	}
	return false
}
