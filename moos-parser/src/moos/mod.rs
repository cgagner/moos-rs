extern crate lalrpop_util;

pub mod error;
pub mod lexer;
pub mod tree;

// lalrpop_mod!(
//   #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
//   pub moos "moos/moos.rs"
// ); // syntesized by LALRPOP

pub mod moos {
    include!(concat!(env!("OUT_DIR"), "/moos/moos.rs"));
}
