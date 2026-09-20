# Entry thin-binary generation

Verifies a recognized `main.ts` entry owns one project library plus one
thin `javascript_binary` over the compiled output (`main.js`) with no
source duplication. Execution reuses the JavaScript wrappers; there is no
`typescript_binary`. Only the exact `main` basename is an entry; other
layouts are an explicit wont-fix per issue #585.
