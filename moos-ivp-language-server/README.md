# MOOS-IvP Language Server

## Features

The MOOS-IvP Language Server *will* support a number of different file
formats that are used in MOOS-IvP Projects. Mainly, they *will* support
MOOS Mission files, IvP Behavior files, and NSPlug template files. 

### NSPlug
  * Semantic Highlighting
    * Keywords: Coloring keywords when used in the appropriate context. For 
      example, only color `#define` directive if it is at the start of the
      line, but not if it appears in a quote. 
    * Include Paths: The `#include` directive is followed by a path to a file
      to be included. This is colorized like a quote/string.
    * Include Tags: The `#include` directive has an option `<some_tag>` token
      that can appear at the end of the line. This relatively new feature will
      only include lines from the specified filed between the `some_tag` tag
      and the next tag (or end of the file). 
    * Primitives: NSPlug treats all values as strings. However, we thought it
      would be nice to colorize values based on their type. The language
      server currently supports floats, integers, booleans, and quotes.
  * Diagnostics
    * Errors: 
      - Unknown directives: NSPlug has only a handful of supported directives
        (E.G. `#include`, `#define`, `#ifdef`). An error will be displayed if
        an unknown directive appears in the file. For example, if you have a 
        typo like `#fidef` or `#inlcude`. 
      - Invalid `#ifdef` or `#ifndef` directives: These directive must have a 
        corresponding `#endif`. Additionally, there must be an `#ifdef` before
        there is an `#elseifdef` and `#ifndef` does not support an 
        `#elseifndef`.Also, `#ifndef` directives do not support conjunctive
        (logical-And) or disjunctive (logical-Or) operators. At least one error
        message will be displayed if the parser detects an invalid structure. 
        In the case of a missing `#endif` two error messages will be displayed:
        one at the end if the file and one at the outer most `#ifdef`.
      - Combining conjunctive and disjunctive in the same condition. 
        NSPLug will display a warning by default and will fail in strict mode
        if it detects invalid conditions. The language server will treat these
        as errors.
    * Warnings: 
      - TODO: Undefined Variable: NSPlug will display a warning if a variable
        is not defined and will fail if running in strict mode. However, since
        many NSPlug variables are defined launch scripts, the language server
        will only ever treat these as warnings. 
      - TODO: Display a warning if an `#include` directive has a path that
        does not exist. This cannot be treated as an error because NSPlug
        has the ability to take in a list of directories (using the `--path=`
        argument) to search for files. That list of directories is typically
        set in a launch script that the language server does not have knowledge
        about.
  * Document Links:
    * Include Paths: The `#include` directive is followed by a path to a file
      to be included. If the path exists, the language server will make a 
      document link so you can `<Control>-Click` on the link to open the
      document. This currently only works if the language server has access
      to the local file system. 
  * Completion:
    * Directives: If the `#` character is type at the start of the line, the
      language server will provide completion for the available NSPLug
      directives. For `#ifdef` and `#ifndef`, if completion is selected, the
      matching `#endif` will automatically be inserted on the line below.
    * TODO: Include Path: After the `#include` directive, if a users requests
      completion, the language server will provide a list of files relative
      to the current file. This currently only work if the language server has
      access to the local file system.
    * TODO: Variables: If the `$(` or `%(` strings are typed, the language
      server will provide a list of variables defined in the current
      mission/workspace along with the closing `)` if needed.


## TODO

### MVP

List of items until we have a minimum viable product for others to start
testing with. 

* [ ] Reorganize repositories
     - moos-rs
       - crates  
          - moos-parser
          - moos
          - moos-geodesy
     - moos-ivp-language-server
       - crates
         - lsp-server-derive-macro
         - xtask
       - editors
         - vscode
         - emacs
         - neovim
       - src
* [ ] Work on CI/CD, packaging, and releases.
* [X] BHV Semantic highlighting
      - [ ] Add support to for inactive regions
      - [ ] Add support for parsing conditions
* [ ] Add the ability to validate files from the command line
* [ ] Includes that are not found should display an error or warning.
* [ ] Includes that are found should recursively check for errors in the
      included files. Display an error if the include has an error.
* [ ] Add/change includes to have definitions
