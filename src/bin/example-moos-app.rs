extern crate moos;

use std::{str, str::FromStr};

use crate::moos::message::{Message, ValueType};

fn main() {
    let mut m: Message = Message::from_string("DEPLOY", "true");

    if let ValueType::String(s) = m.value() {
        assert_eq!(s, "true");
    }
    //m.key().push_str("Test");

    let sparkle_heart = vec![240, 159, 146, 150];
    //let sparkle_heart = str::from_utf8(&sparkle_heart).unwrap();
    let test_string = "test";

    //m.key() = String::from(str::from_utf8(&sparkle_heart).unwrap());

    println!("Hello, world!");
}
