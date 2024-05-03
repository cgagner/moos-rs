use lexers::TokenRange;

#[macro_use]
extern crate lalrpop_util;

pub mod ast;
pub mod helpers;
pub mod lexers;
pub mod moos;
pub mod nsplug;

/// Type of Str used in parse trees. This type needs to own the str
/// to get around issues with the borrow checker. This type could be changed
/// to a `Arc<str>`, `Rc<str>`, or `Box<str>` depending on the need for thread
/// safety.
#[cfg(not(feature = "threadsafe-tree"))]
pub type TreeStr = Box<str>;

/// Type of Str used in parse trees. This type needs to own the str
/// to get around issues with the borrow checker. This type could be changed
/// to a `Arc<str>`, `Rc<str>`, or `Box<str>` depending on the need for thread
/// safety.
#[cfg(feature = "threadsafe-tree")]
pub type TreeStr = std::sync::Arc<str>;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type MoosParser = moos::moos::LinesParser;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type MoosLexer<'input> = moos::lexer::Lexer<'input>;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type PlugLexer<'input> = nsplug::lexer::Lexer<'input>;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type PlugParser = nsplug::nsplug::LinesParser;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type ParseError<L, T, E> = lalrpop_util::ParseError<L, T, E>;

pub trait TreeNode: ToString {
    /// Get the range in the current line for the node
    #[inline]
    fn get_range(&self) -> lexers::TokenRange {
        lexers::TokenRange {
            start: self.get_start_index(),
            end: self.get_end_index(),
        }
    }

    /// Get the start index in the current line for the node
    fn get_start_index(&self) -> u32;

    /// Get the end index in the current line for the node
    fn get_end_index(&self) -> u32;

    /// Check if the specified index is inside the range of this node.
    /// This will return true if the index is equal to the start or end index
    /// as well.
    #[inline]
    fn is_inside(&self, index: u32) -> bool {
        index >= self.get_start_index() && index <= self.get_end_index()
    }
}

#[derive(Debug)]
pub struct Quote<V: ToString> {
    pub content: V,
    pub range: TokenRange,
}

impl<V: ToString> Quote<V> {
    fn get_token_range(&self) -> &TokenRange {
        &self.range
    }
}

impl<V: ToString> TreeNode for Quote<V> {
    /// Get the start index in the line for the value
    #[inline]
    fn get_start_index(&self) -> u32 {
        self.range.start
    }

    /// Get the end index in the line for the value
    #[inline]
    fn get_end_index(&self) -> u32 {
        self.range.end
    }
}

impl<V: ToString> ToString for Quote<V> {
    fn to_string(&self) -> String {
        return format!("\"{}\"", self.content.to_string());
    }
}

#[cfg(feature = "lsp-types")]
fn create_text_edit(
    new_text: String,
    line: u32,
    start_index: u32,
    end_index: u32,
) -> lsp_types::TextEdit {
    lsp_types::TextEdit {
        range: lsp_types::Range {
            start: lsp_types::Position {
                line,
                character: start_index,
            },
            end: lsp_types::Position {
                line: line,
                character: end_index,
            },
        },
        new_text,
    }
}

pub struct FormatOptions {
    pub insert_spaces: bool,
    pub tab_size: u32,
}

#[cfg(feature = "lsp-types")]
impl From<lsp_types::FormattingOptions> for FormatOptions {
    fn from(value: lsp_types::FormattingOptions) -> Self {
        FormatOptions {
            insert_spaces: value.insert_spaces,
            tab_size: value.tab_size,
        }
    }
}

#[cfg(feature = "lsp-types")]
impl From<&lsp_types::FormattingOptions> for FormatOptions {
    fn from(value: &lsp_types::FormattingOptions) -> Self {
        FormatOptions {
            insert_spaces: value.insert_spaces,
            tab_size: value.tab_size,
        }
    }
}

#[cfg(feature = "lsp-types")]
pub trait TextFormatter {
    /// Create TextEdits for the macros. This should only manipulate the
    /// whitespace in the line.
    fn format(&self, format_options: &FormatOptions, level: u32) -> Vec<lsp_types::TextEdit>;
}

#[cfg(test)]
mod tests {

    use crate::{PlugLexer, PlugParser};

    #[test]
    fn test_plug_parser() -> anyhow::Result<()> {
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
