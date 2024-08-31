extern crate lalrpop_util;

pub mod error;
pub mod lexer;
pub mod tree;

// lalrpop_mod!(
//   #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
//   pub behavior "behavior/behavior.rs"
// ); // syntesized by LALRPOP

pub mod behavior {
    include!(concat!(env!("OUT_DIR"), "/behavior/behavior.rs"));
}
