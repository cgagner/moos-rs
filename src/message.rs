// message.rs
// 

use core::mem;
use std::io::Error;

pub struct Message {
    pub length: i32, // @TODO: the length should probably go away since it is just calculated
    pub id: i32,
    pub message_type: MessageType,
    pub data_type: DataType,
    pub double_value: f64,
    pub double_value2: f64,
    pub string_value: String,
    pub key: String,
    pub time: f64,
    pub source: String,
    pub source_aux: String,
    pub originating_community: String,
}

type MessageList = Vec<Message>;

/*
 * length (i32)
 * id (i32)
 * message_type (i8)
 * data_type (i8)
 * source [u8] (std::string)
 * source_aux [u8] (str::string)
 * originating_community [u8] (std::string)
 * key [u8] (std::string)
 * time (f64)
 * double_value (f64)
 * double_value2 (f64)
 * string_value - [u8]
 *
 */
impl Message {
    pub fn new<S>(message_type: MessageType, key: S) -> Self
    where
        S: Into<String>,
    {
        Message {
            length: 0,                  //
            id: 0,                      //
            message_type: message_type, //
            data_type: DataType::Double,
            double_value: 0.0,
            double_value2: 0.0,
            string_value: String::new(),
            time: 0.0,
            key: key.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    // @TODO: Need a encode_string and decode_string
    pub fn encode_string(buffer: &mut [u8], s: &str) -> usize {
        0
    }

    fn decode_string(buffer: &[u8]) -> String {
        String::new()
    }

    /// Returns the size of the message when serialized
    pub fn get_size(&self) -> i32 {
        // (mem::size_of(self.id) +
        //mem::size_of(self.message_type)) as i32

        (mem::size_of_val(&self.id) 
        + mem::size_of_val(&self.double_value)) as i32
        
    }


}

/// Type of the message.
#[derive(PartialEq, Eq, Debug)]
pub enum MessageType {
    Null,
    Anonymous,
    Command,
    Poision,
    Notify,
    ServerRequest,
    Register,
    Unregister,
    WildcardRegister,
    WildcardUnregister,
    Welcome,
    Data,
    NotSet,
    Timing,
    TerminateConnection,
    ServerRequestId,
}



impl MessageType {
    /// Create a MessageType from a byte.
    pub const fn from_byte(value: i8) -> MessageType {
        match value {
            0x2E => MessageType::Null,
            0x41 => MessageType::Anonymous,
            0x43 => MessageType::Command,
            0x4B => MessageType::Poision,
            0x4E => MessageType::Notify,
            0x51 => MessageType::ServerRequest,
            0x52 => MessageType::Register,
            0x55 => MessageType::Unregister,
            0x2A => MessageType::WildcardRegister,
            0x2F => MessageType::WildcardUnregister,
            0x57 => MessageType::Welcome,
            0x69 => MessageType::Data,
            0x7E => MessageType::NotSet,
            0x54 => MessageType::Timing,
            0x5E => MessageType::TerminateConnection,
            -2 => MessageType::ServerRequestId,
            _ => MessageType::Null,
        }
    }

    /// Get the byte value of the MessageType.
    pub fn to_byte(&self) -> i8 {
        match *self {
            MessageType::Null => '.' as i8,
            MessageType::Anonymous => 'A' as i8,
            MessageType::Command => 'C' as i8,
            MessageType::Poision => 'K' as i8,
            MessageType::Notify => 'N' as i8,
            MessageType::ServerRequest => 'Q' as i8,
            MessageType::Register => 'R' as i8,
            MessageType::Unregister => 'U' as i8,
            MessageType::WildcardRegister => '*' as i8,
            MessageType::WildcardUnregister => '/' as i8,
            MessageType::Welcome => 'W' as i8,
            MessageType::Data => 'i' as i8,
            MessageType::NotSet => '~' as i8,
            MessageType::Timing => 'T' as i8,
            MessageType::TerminateConnection => '^' as i8,
            MessageType::ServerRequestId => -2 as i8,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum DataType {
    Double,
    String,
    Binary,
}

impl DataType {
    /// Create a DataType from a byte.
    pub fn from_byte(value: i8) -> DataType {
        match value {
            0x53 => DataType::String,
            0x44 => DataType::Double,
            0x42 => DataType::Binary,
            _ => DataType::String,
        }
    }

    /// Get the byte value of the DataType.
    pub fn to_byte(&self) -> i8 {
        match *self {
            DataType::Binary => 'B' as i8,
            DataType::Double => 'D' as i8,
            DataType::String => 'S' as i8,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_data_type() {
        use crate::comms::DataType;
        assert_eq!(
            DataType::Binary,
            DataType::from_byte(DataType::Binary.to_byte())
        );
        assert_eq!(
            DataType::Double,
            DataType::from_byte(DataType::Double.to_byte())
        );
        assert_eq!(
            DataType::String,
            DataType::from_byte(DataType::String.to_byte())
        );
    }
}

struct Packet {}

impl Packet {
    pub fn encode(message_list: MessageList) -> Result<bool, Error> {
        let buffer_size: i32 = message_list.iter().map(|message| message.get_size()).sum();

        let mut num_messages: u32 = 0;
        let mut byte_count: u32 = 0;
        let mut offset: u32 = 0;
        for message in message_list.iter() {
            num_messages += 1;
            // let num_bytes = match message.serialize() {
            //     Ok(b) => b,
            //     Err(e) => {
            //         println!("Packet::encode(): failed to encode");
            //         return Err(Error::SerializationFailure);
            //     }
            // };
        }

        Ok(true)
    }
}
