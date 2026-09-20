// ring-style C/assembly shape stub (issue #499).
#pragma once

// C plus assembly with target-platform libraries. Build scripts keep
// execution-platform tools while applications link target libs. This stub
// proves the C closure; the assembly dialect plus target-libs pin lives
// in pins.bzl with the backend provisional.
#ifdef __cplusplus
extern "C" {
#endif

int dx_ring_hash(const char *input, char out_hex[65]);
int dx_ring_asm_add(int a, int b);

#ifdef __cplusplus
}
#endif
