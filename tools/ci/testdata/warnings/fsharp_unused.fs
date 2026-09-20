// Warning fixture: unused value (F# warnaserror+).
module WarningUnused

let greet (name: string) =
    let unused = 1
    "hello " + name
