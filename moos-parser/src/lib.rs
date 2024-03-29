#[macro_use]
extern crate lalrpop_util;

pub mod ast;
pub mod error;
pub mod helpers;
pub mod lexer;
pub mod lexers;
pub mod nsplug;
pub mod parser;

use lalrpop_util::ErrorRecovery;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type Lexer<'input, 'listen> = lexer::Lexer<'input, 'listen>;

lalrpop_mod!(
    #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
    pub moos
); // syntesized by LALRPOP

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type LinesParser = moos::LinesParser;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type PlugLexer<'input, 'listen> = nsplug::lexer::Lexer<'input, 'listen>;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type PlugParser = nsplug::nsplug::LinesParser;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type ParseError<L, T, E> = lalrpop_util::ParseError<L, T, E>;

#[cfg(test)]
mod tests {

    use crate::{PlugLexer, PlugParser};

    #[test]
    fn test_plug_parser() -> anyhow::Result<()> {
        use crate::PlugParser;

        use tracing::level_filters::LevelFilter;
        use tracing_subscriber::fmt::writer::BoxMakeWriter;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::{fmt, EnvFilter, Registry};
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env()?
            .add_directive("moos_parser=trace".parse()?);
        let writer = BoxMakeWriter::new(std::io::stderr);
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_ansi(false)
            .with_filter(filter);

        Registry::default().with(fmt_layer).try_init()?;

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

        let input = "// This is a test\n#define FOO Appless // Comment \n\n\n\n";

        let lexer = PlugLexer::new(input);
        let mut state = crate::nsplug::lexer::State::default();
        let result = PlugParser::new().parse(&mut state, input, lexer);

        println!("Result: {:?}", result);

        assert!(result.is_ok());

        Ok(())
    }
}
