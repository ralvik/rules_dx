// OpenSSL shape stub with declared build tools (issue #499).
#include "cc/tests/fixtures/linux_corpus/openssl.h"

#include <stddef.h>

int dx_openssl_version(void) {
  return 30000000;
}

int dx_openssl_digest(const char *input, char out_hex[65]) {
  static const char kHex[] = "0123456789abcdef";
  unsigned long long h = 1469598103934665603ULL;
  if (input == NULL || out_hex == NULL) {
    return 1;
  }
  while (*input != '\0') {
    h ^= (unsigned long long)(unsigned char)(*input);
    h *= 1099511628211ULL;
    ++input;
  }
  for (int i = 0; i < 32; ++i) {
    out_hex[i * 2] = kHex[(h >> (4 * (i % 16))) & 0xF];
    out_hex[i * 2 + 1] = kHex[(h >> (4 * ((i + 7) % 16))) & 0xF];
  }
  out_hex[64] = '\0';
  return 0;
}
