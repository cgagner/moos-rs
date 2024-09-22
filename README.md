# MOOS Rust

This project is being refactored to utilize Rust workspaces. This will allow
for multiple crates to be maintained within this repository. The following
crates will be coming soon:

- `moos`: Pure Rust implementation of a MOOS client/application library. 
- `moos-parser`: Pure Rust implementation of a MOOS mission file, IvP
   Behavior, and NSPlug parsers.
- `moos-geodesy`: Rust version of MOOS-Geodesy.
- `moos-language-server`: Implementation of a language server for MOOS mission
   files. This will allow for creating a VSCode extension for MOOS mission
   files. The language server *could* also be used for other editors such
   as `vim` and `emacs`, but those editors are not on the current roadmap.

For additional information, please contact contact christoper.gagner@gmail.com
