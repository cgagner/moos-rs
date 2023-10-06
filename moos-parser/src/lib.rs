#[macro_use]
extern crate lalrpop_util;

pub mod ast;
pub mod error;
pub mod helpers;
pub mod lexer;
pub mod parser;

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type Lexer<'input, 'listen> = lexer::Lexer<'input, 'listen>;

use lalrpop_util::ErrorRecovery;
lalrpop_mod!(
    #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
    pub moos
); // syntesized by LALRPOP

#[allow(clippy::all, dead_code, unused_imports, unused_mut)]
pub type LinesParser = moos::LinesParser;
