# MOOS Parser

Parser for MOOS-IvP mission files for Rust.

## TODO

- [x] Fix the `_next()` method. Currently just returns None. Requires
      `tokenize()` to be called first.
- [ ] Update Float, Integer, Bool tokens to have both the string and primitive
      value.
- [ ] Assigning a variable with one character seems to break
- [ ] Switch to scanning the whole line - Makes things easier.
- [ ] The lexer currently returns errors. This prevents the parser from recovering. See
      lalrpop start_machine.rs line 611.
- [ ] If macros are used for blocks (e.g. ProcessConfig) we need to check
      that each branch has a process config. That may also screw up the
      vs-code plugin since there may be different process configs.
- [ ] Remove the scan_identifier method and return a constant ValueString
- [ ] Replace the Key/Identifier from moos.lalrpop with ValueString
- [ ] Need to handle variables in the middle of a string
- [ ] Variables can also appear in a comment
- [ ] Support "#define $(FOO) BAR" as well as "#define FOO BAR"
- [ ] Missing New Line needs a better comment - Currently always reports "Need new line after application name"
- [ ] Handle lines that start with # but are not macros
- [ ] Handle keywords based on MOOS file or BHV file
- [ ] Need to handle behaviors files differently
  - [ ] Behaviors support # as comments
  - [ ] Behaviors don't support 'define:'
  - [ ] Behaviors don't support environment variables
  - [ ] Behaviors can be static or dynamic 
- [ ] Fix the test_scan_variables
- [ ] Remove the handling of single quotes
- [ ] Check for Plug variables in comments
- [ ] Finish Marco parsing in lalrpop
  - [ ] Add MacroIfDef
  - [ ] Add MacroElseIfDef
  - [ ] Add MacroIfNotDef
- [ ] Update the Assignment type to take in a vector of tokens
- [ ] Update Float, Int, Bool tokens to also take the original str
- [ ] Add a warning to highlight `name=<empty_string>`
- [ ] Add the ability to throw an error on a double `{` See PCR 114


## nsplug Questions

1. Is it intended that `nsplug` doesn't support comments after macros:
      ```text
      #ifdef VALUE 12 // Test Comment
      ```
1. Why does `ifndef` without a variable result in an error?
1. Should spaces in between `#` and a macro be an error? E.G. `#   ifdef`
1. Do variables in quotes get replaced? E.G. `value = "${VAR}"`
1. It looks like `%(VAL)` replaces with the uppercase version of VAL where `$(VAL)`
   just replaces VAL verbatim.
