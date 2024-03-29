extern crate lalrpop_util;

pub mod error;
pub mod lexer;
pub mod tree;

use lalrpop_util::ErrorRecovery;

// lalrpop_mod!(
//   #[allow(clippy::all, dead_code, unused_imports, unused_mut)]
//   pub nsplug "nsplug/nsplug.rs"
// ); // syntesized by LALRPOP

pub mod nsplug {
    include!(concat!(env!("OUT_DIR"), "/nsplug/nsplug.rs"));
}
