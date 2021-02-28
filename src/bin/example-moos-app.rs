extern crate moos;

use std::str;

use crate::moos::message::Message;

fn main() {
    use moos::message::{MessageType};

    let key = String::from("DEPLOY");

    let mut m : Message = Message::new(MessageType::Data, key);
    
    m.key.push_str("Test");

    let sparkle_heart = vec![240, 159, 146, 150];
    //let sparkle_heart = str::from_utf8(&sparkle_heart).unwrap();
    let test_string = "test";
    
    m.key = String::from(str::from_utf8(&sparkle_heart).unwrap());

    println!("Hello, world!");
}
