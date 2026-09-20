// Bindgen LLVM-22-vs-23 compat fixture header.
//
// Minimal C11 header exercising only constructs whose bindings are stable
// across the pinned LLVM-22 parser baseline (hermetic-llvm 0.8.18,
// LLVM 22.1.8 via rules_rs v0.0.109) and the LLVM-23 target flags
// (hermetic-llvm v0.8.19, LLVM 23.1.0). Both the standalone `rust_bindgen`
// route (bindgen 0.72.1 prebuilt v0.0.2, `--no-include-path-detection`
// `--formatter=none` with target compiler context) and the build-script
// route (explicit execution-platform libclang closure plus target parsing
// flags) must produce exactly the `bindgen.expected` symbol set from this
// header. Deliberately avoids C23-only spellings (`typeof`, `constexpr`,
// `_BitInt`, `#embed`), modules, VLAs in the public API, and
// platform-conditional preprocessing: those need separate per-target
// qualification and are not covered by this compat proof. Unpinned LLVM
// (floating `llvm.version(...)` or untracked hermetic-llvm) is rejected:
// parser/header skew breaks determinism, so both LLVM identities stay
// pinned in `pins.bzl` and `docs/native-toolchains.md`.
#ifndef DX_BINDGEN_H
#define DX_BINDGEN_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define DX_BINDGEN_MAX_LABELS 16

typedef enum DxStatus {
  DX_STATUS_OK = 0,
  DX_STATUS_EMPTY = 1,
  DX_STATUS_BAD_ARG = 2,
} DxStatus;

typedef struct DxPoint {
  int32_t x;
  int32_t y;
} DxPoint;

typedef struct DxTagged {
  int32_t kind;
  union {
    struct {
      int32_t x;
      int32_t y;
    } point;
    struct {
      int32_t w;
      int32_t h;
    } size;
  };
} DxTagged;

typedef double (*DxMeasureFn)(const DxPoint *a, const DxPoint *b);

typedef struct DxStore DxStore;

double dx_distance(const DxPoint *a, const DxPoint *b);
int32_t dx_tagged_area(const DxTagged *value);
DxStore *dx_store_create(void);
void dx_store_destroy(DxStore *store);
size_t dx_store_len(const DxStore *store);
const char *dx_status_message(DxStatus status);

#endif /* DX_BINDGEN_H */
