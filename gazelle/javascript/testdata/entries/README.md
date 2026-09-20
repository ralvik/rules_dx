# Entry thin-binary generation

Verifies a recognized `main.js` entry owns one library plus one thin
binary with no source duplication. Only the exact `main` basename is an
entry; other layouts are an explicit wont-fix per issue #585.
