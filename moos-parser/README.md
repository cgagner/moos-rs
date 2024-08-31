# MOOS Parser

Parser for MOOS-IvP mission files for Rust.

## TODO
- [ ] If macros are used for blocks (e.g. ProcessConfig) we need to check
      that each branch has a process config. That may also screw up the
      vs-code plugin since there may be different process configs.
- [ ] Support "#define $(FOO) BAR" as well as "#define FOO BAR"
- [ ] Handle keywords based on MOOS file or BHV file
- [ ] Need to handle behaviors files differently
  - [ ] Behaviors support # as comments
  - [ ] Behaviors don't support 'define:'
  - [ ] Behaviors don't support environment variables
  - [ ] Behaviors can be static or dynamic 
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
