# MOOS Rust

This project is being refactored to utilize Rust workspaces. This will allow
for multiple crates to be maintained within this repository. The following
crates will be coming soon:

- `moos`: Pure Rust implementation of a MOOS client/application library. 
- `moos-parser`: Pure Rust implementation of a MOOS mission file parser.
- `moos-language-server`: Implementation of a language server for MOOS mission
   files. This will allow for creating a VSCode extension for MOOS mission
   files. The language server *could* also be used for other editors such
   as `vim` and `emacs`, but those editors are not on the current roadmap.

For additional information, please contact contact christoper.gagner@gmail.com

## MOOS Labs Rust Tasks

MOOS Features:
  - MOOS Application
  - App casting

* Lab 01: Machine Setup
  - [ ] Add a section for setting up Rust
* Lab 02: Introduction to Rust
  - [ ] Simple introduction. Point to Rust book.
* 
* Lab 04: Intro to MOOS
  - [ ] MOOS App structure in Rust
  - [ ] Functions on MOOS Msg
  - [ ] `moos-ivp-extend`
  - [ ] `pOdometry`
  - [ ] Template. Possibly use Cargo to create. `cargo-generate`.
  - [ ] Alder mission
  - [ ] Setup logging


