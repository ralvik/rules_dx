// ring-style C/assembly shape stub (issue #499).
#include "cc/tests/fixtures/linux_corpus/ring.h"

#include <stddef.h>

int dx_ring_hash(const char *input, char out_hex[65]) {
  static const char kHex[] = "0123456789abcdef";
  unsigned long long h = 1099511628211ULL;
  if (input == NULL || out_hex == NULL) {
    return 1;
  }
  while (*input != '\0') {
    h += (unsigned long long)(unsigned char)(*input);
    h *= 31ULL;
    ++input;
  }
  for (int i = 0; i < 32; ++i) {
    out_hex[i * 2] = kHex[(h >> (4 * (i % 16))) & 0xF];
    out_hex[i * 2 + 1] = kHex[(h >> (4 * ((i + 3) % 16))) & 0xF];
  }
  out_hex[64] = '\0';
  return 0;
}

// Stands in for the assembly fast path; the dialect plus target-libs
// requirement is pinned, the C closure is proven here.
int dx_ring_asm_add(int a, int b) {
  return a + b;
}
