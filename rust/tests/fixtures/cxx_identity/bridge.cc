// CXX identity bridge source stub (issue #474).
// Stands in for cxxbridge-cmd output until corpus wiring lands under #499.
#include "rust/tests/fixtures/cxx_identity/bridge.h"

int bridge_add(int left, int right) {
    return left + right;
}
