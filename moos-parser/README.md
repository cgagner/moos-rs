# MOOS Parser

Parser for MOOS-IvP mission files for Rust.

## TODO
- [ ] If macros are used for blocks (e.g. ProcessConfig) we need to check
      that each branch has a process config. That may also screw up the
      vs-code plugin since there may be different process configs.
- [ ] Support "#define $(FOO) BAR" as well as "#define FOO BAR"
- [X] Handle keywords based on MOOS file or BHV file
- [X] Need to handle behaviors files differently
  - [ ] ~~Behaviors support # as comments~~ See Populate_BehaviorSet
  - [X] Behaviors don't support 'define:'
  - [ ] Need to handle initialize with multiple assignments
  - [ ] Behaviors don't support environment variables
  - [ ] Add support for parsing/validating conditions
  - [ ] Behaviors support special variables with square brackets
    - [ ] Waypoint Behavior
      - $[X] Expanded to ownship’s x position in local coordinates.
      - $[Y] Expanded to ownship’s x position in local coordinates
      - $[NX] Expanded to the x position of the next waypoint in local coordinates.
      - $[NY] Expanded to the y position of the next waypoint in local coordinates.
    - [ ] Contact Flag (`cnflag`)
      - Triggers
        - @cpa: When the closes point of approach is observed
        - @os passes cn: When ownship passes contact’s beam
        - @os passes cn port When ownship passes contact’s port beam
        - @os passes cn star: When ownship passes contact’s starboard beam
        - @cn passes os When ownship passes contact’s beam
        - @cn passes os port When contact passes contact’s port beam
        - @cn passes os star When contact passes contact’s starboard beam
        - @os crosses cn When ownship crosses contact’s side
        - @os crosses cn bow When ownship crosses contact’s side fore of contact
        - @os crosses cn stern When ownship crosses contact’s side aft of contact
        - @cn crosses os When contact crosses ownship’s side
        - @cn crosses os bow When contact crosses ownship’s side fore of ownship
        - @cn crosses os stern When contact crosses ownship’s side aft of ownship
      - Macros
        - $[RANGE] Range between ownship and contact
        - $[CN NAME] Name of the contact
        - $[CN GROUP] Name of the contact
        - $[CN VTYPE] Vehicle type of the contact
        - $[ROC] Rate of Closure between ownship and the contact
        - $[OS CN REL BNG] Relative bearing of the contact to ownship
        - $[CN OS REL BNG] Relative bearing of ownship to the contact
        - $[BNG RATE] Bearing Rate
        - $[CN SPD IN OS POS] Speed of contact in the direction of ownship position
        - $[OS FORE OF CN] true if ownship is currently fore of the contact
        - $[OS AFT OF CN] true if ownship is currently aft of the contact
        - $[OS PORT OF CN] true if ownship is currently on port side of the contact
        - $[OS STAR OF CN] true if ownship is currently on starboard side of the contact
        - $[CN FORE OF OS] true if the contact is currently fore of ownship
        - $[CN AFT OF OS] true if the contact is currently aft of ownship
    - [ ] Collision Avoidance
      - Macros
        - $[CONTACT]
        - $[VNAME]
    - [ ] Convoy
      - Macros
        - $[ALIGNMENT]
        - $[RECAP]
        - $[CONVOY_RNG]
  - [ ] Behaviors can be static or dynamic 
- [X] Add a warning to highlight `name=<empty_string>`
- [ ] Add the ability to throw an error on a double `{` See PCR 114


## nsplug Questions

1. Is it intended that `nsplug` doesn't support comments after macros:
      ```text
      #ifdef VALUE 12 // Test Comment
      ```
1.~~ Why does `ifndef` without a variable result in an error?~~
1. ~~Should spaces in between `#` and a macro be an error? E.G. `#   ifdef`~~
1. ~~Do variables in quotes get replaced? E.G. `value = "${VAR}"`~~
1. It looks like `%(VAL)` replaces with the uppercase version of VAL where `$(VAL)`
   just replaces VAL verbatim.
