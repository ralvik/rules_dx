// M22 seed C++ binary; consumer of cc_binary.
#include <iostream>

#include "cc/tests/fixtures/hello/hello.h"

int main() {
  std::cout << "hello " << Add(40, 2) << "\n";
  return 0;
}
