// Seed C test; consumer of cc_test.
#include <assert.h>

#include "cc/tests/fixtures/hello/hello_c.h"

int main(void) {
  assert(AddC(2, 3) == 5);
  assert(AddC(-1, 1) == 0);
  return 0;
}
