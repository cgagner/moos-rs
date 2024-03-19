#[macro_use]
extern crate lalrpop_util;

pub mod ast;
pub mod error;
pub mod helpers;
pub mod lexer;
pub mod parser;
pub(crate) mod nsplug_tree;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type Lexer<'input, 'listen> = lexer::Lexer<'input, 'listen>;

use lalrpop_util::ErrorRecovery;
lalrpop_mod!(
    #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
    pub moos
); // syntesized by LALRPOP

lalrpop_mod!(
    #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
    pub nsplug
); // syntesized by LALRPOP

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type LinesParser = moos::LinesParser;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type PlugParser = nsplug::DirectivesParser;

#[cfg(test)]
mod tests {

    use crate::PlugParser;

    #[test]
    fn test_plug_parser() {
        use crate::PlugParser;

        let input = r#"#include <test.plug>
        // Test Comment
        #define MY_VARIABLE 1234
        $(MY_VARIABLE)
        #include "test.plug"


        #ifdef ASDF

        

        // MIT Sailing Pavilion
        // test
        // test
        #include "asdf.plug"
        LatOrigin  = 42.35846207515723
        LongOrigin = -71.08774014042629

        //------------------------------------------
        // Antler configuration  block
        ProcessConfig = ANTLER
        {
          MSBetweenLaunches = 200
          ExecutablePath = system // System path
          Run = MOOSDB          @ NewConsole = false
          Run = pLogger         @ NewConsole = true
          Run = uSimMarine      @ NewConsole = false
          Run = pMarinePID      @ NewConsole = false
          Run = pHelmIvP        @ NewConsole = true, ExtraProcessParams=HParams
          Run = pMarineViewer	@ NewConsole = false
          Run = uProcessWatch	@ NewConsole = false
          Run = pNodeReporter	@ NewConsole = false
          Run = uMemWatch       @ NewConsole = false
          Run = pXRelay @ NewConsole = true ~ pXRelay_PEARS

          // Helm Params
          HParams=--alias=pHelmIvP_Standby
        }
        define: MY_VAR = "this is a test"
        //------------------------------------------
        // uMemWatch config block

        "#;

        let input = r#"#include <test.plug>
        // Test Comment
        #define MY_VARIABLE 1234
        $(MY_VARIABLE)
        #include "test.plug"
        
        #ifdef JJJJJJ 12345
        #elseifdef JJKK
        #else
        #endif

        #ifdef ASDF
        name = value
        
        #include "test.txt"
        #define AS
        
        #endif

        "#;

        let mut start_of_file = true;

        let result = PlugParser::new().parse(&mut start_of_file, input);

        println!("Result: {:?}", result);

        assert!(result.is_ok());
    }
}
