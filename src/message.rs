// message.rs
// 

use core::mem;
use std::{io::Error, str::from_utf8};
use core::convert::TryInto;
use crate::errors::{InsufficientSpaceError, Result};

use super::errors;


pub struct Message {
    pub id: i32,
    pub message_type: MessageType,
    pub data_type: DataType,
    pub double_value: f64,
    pub double_value2: f64,
    // Switch from using string to a Vec<u8> to handle 
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
 * source (i32) (std::string)
 * source_aux (i32) (str::string)
 * originating_community (i32) (std::string)
 * key (i32) (std::string)
 * time (f64)
 * double_value (f64)
 * double_value2 (f64)
 * string_value - (i32) (std::string)
 *
 */
impl Message {
    pub fn new<S>(message_type: MessageType, key: S) -> Self
    where
        S: Into<String>,
    {
        Message {
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


    /// Decode a Message from a [u8] slice
    ///
    /// 
    pub fn decode_slice(&mut self, buffer: &[u8]) -> errors::Result<usize> {
        let mut reader = Reader::new(buffer);

        let length = reader.read_i32()?;
        let id = reader.read_i32()?;
        let message_type = MessageType::from_byte(reader.read_i8()?);
        let data_type = DataType::from_byte(reader.read_i8()?);
        let source = reader.read_string()?;
        let source_aux = reader.read_string()?;
        let originating_community = reader.read_string();
        let key = reader.read_string();
        let time = reader.read_f64();
        let double_value = reader.read_f64();
        let double_value2 = reader.read_f64();
        let string_value = reader.read_string();
        
        Ok(reader.bytes_read)
    }

    pub fn encode_slice(&self, buffer: &mut [u8]) -> errors::Result<usize> {
        let len = self.get_size();
        // Check that the buffer has enough size to store the message
        if buffer.len() < (len as usize) {
            return Err(InsufficientSpaceError);
        }
        let mut writer = Writer::new(buffer);
        
        writer.write_i32(len)?;
        writer.write_i32(self.id)?;
        writer.write_i8(self.message_type.to_byte())?;
        writer.write_i8(self.data_type.to_byte())?;
        writer.write_string(&self.source)?;
        writer.write_string(&self.source_aux)?;
        writer.write_string(&self.originating_community)?;
        writer.write_string(&self.key)?;
        writer.write_f64(self.time)?;
        writer.write_f64(self.double_value)?;
        writer.write_f64(self.double_value2)?;
        // TODO: This should check the type and write a vector if it is binary
        writer.write_string(&self.string_value)?;

        Ok(writer.bytes_written)
    }

    /// Returns the size of the message when serialized
    pub fn get_size(&self) -> i32 {
        (
            // TODO: Need to add the length here?
            mem::size_of_val(&self.id) 
             + mem::size_of::<i8>() // message type 
             + mem::size_of::<i8>() // data type 
             + mem::size_of::<i32>() + self.source.len()
             + mem::size_of::<i32>() + self.source_aux.len() 
             + mem::size_of::<i32>() + self.string_value.len()
             + mem::size_of::<i32>() + self.key.len()
             + mem::size_of_val(&self.time)
             + mem::size_of_val(&self.double_value)
             + mem::size_of_val(&self.double_value2)
             + 1 + self.string_value.len()
        ) as i32
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



enum Endian {
    LittleEndian,
    BigEndian,
    NativeEndian,
}

struct Reader<'a> {
    bytes_read: usize,
    buffer: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Create a new reader for a byte buffer
    fn new(buffer: &'a [u8]) ->  Self {
        Reader{bytes_read: 0, buffer}
    }

    fn read_i8(&mut self) -> errors::Result<i8> {
        if self.buffer.len() - self.bytes_read < core::mem::size_of::<i8>() {
            return Err(InsufficientSpaceError);
        }
        let value = self.buffer[self.bytes_read] as i8;
        self.bytes_read += core::mem::size_of::<i8>();
        Ok(value)
    }

    fn read_i32(&mut self) -> errors::Result<i32> {
        if self.buffer.len() - self.bytes_read < core::mem::size_of::<i32>() {
            return Err(InsufficientSpaceError);
        }
        let buf: &[u8; core::mem::size_of::<i32>()] = match self.buffer[self.bytes_read..(self.bytes_read + core::mem::size_of::<i32>())].try_into() {
            Ok(buf) => buf,
            Err(e) => return Err(errors::Error::Conversion(e)),
        };
        let value = i32::from_le_bytes(*buf);
        self.bytes_read += core::mem::size_of::<i32>();
        Ok(value)
    }

    fn read_f64(&mut self) -> errors::Result<f64> {
        if self.buffer.len() - self.bytes_read < core::mem::size_of::<f64>() {
            return Err(InsufficientSpaceError);
        }
        let buf: &[u8; core::mem::size_of::<f64>()] = match self.buffer[self.bytes_read..(self.bytes_read + core::mem::size_of::<f64>())].try_into() {
            Ok(buf) => buf,
            Err(e) => return Err(errors::Error::Conversion(e)),
        };
        let value = f64::from_le_bytes(*buf);
        self.bytes_read += core::mem::size_of::<f64>();
        Ok(value)
    }

    fn read_string(&mut self) -> errors::Result<String> {
        let length = self.read_i32()? as usize;
        if self.buffer.len() - self.bytes_read < length {
            return Err(InsufficientSpaceError);
        }
        let s = match std::str::from_utf8(&self.buffer[self.bytes_read..(self.bytes_read + length)]) {
            Ok(s) => s,
            Err(e) => return Err(errors::Error::Utf8(e)),
        };
        self.bytes_read += length;
        Ok(String::from(s))
    }

    fn read_vector(&mut self) -> errors::Result<Vec<u8>> {
        let length = self.read_i32()? as usize;
        if self.buffer.len() - self.bytes_read < length {
            return Err(InsufficientSpaceError);
        }
        let mut v = Vec::new();
        v.extend_from_slice(&self.buffer[self.bytes_read..(self.bytes_read + length)]);
        self.bytes_read += length;
        Ok(v)
    }
}

struct Writer<'a> {
    bytes_written: usize,
    buffer: &'a mut [u8],
}

impl <'a> Writer<'a> {
    fn new(buffer: &'a mut [u8]) ->  Self {
        Writer{bytes_written: 0, buffer}
    }

    fn write_i8(&mut self, value: i8) -> errors::Result<usize> {
        if self.bytes_written + core::mem::size_of::<i8>() > self.buffer.len(){
            return Err(errors::InsufficientSpaceError);
        }
        self.buffer[self.bytes_written] = value as u8;
        self.bytes_written += core::mem::size_of::<i8>();
        Ok(core::mem::size_of::<i8>())
    }

    fn write_i32(&mut self, value: i32) -> errors::Result<usize> {
        if self.bytes_written + core::mem::size_of::<i32>() > self.buffer.len(){
            return Err(errors::InsufficientSpaceError);
        }
        self.buffer[self.bytes_written..(self.bytes_written + core::mem::size_of::<i32>())].copy_from_slice(&value.to_le_bytes());
        self.bytes_written += core::mem::size_of::<i32>();
        Ok(core::mem::size_of::<i32>())
    }

    fn write_f64(&mut self, value: f64) -> errors::Result<usize> {
        if self.bytes_written + core::mem::size_of::<f64>() > self.buffer.len(){
            return Err(errors::InsufficientSpaceError);
        }
        self.buffer[self.bytes_written..(self.bytes_written + core::mem::size_of::<f64>())].copy_from_slice(&value.to_le_bytes());
        self.bytes_written += core::mem::size_of::<f64>();
        Ok(core::mem::size_of::<f64>())
    }

    fn write_string(&mut self, value: &str) -> errors::Result<usize> {
        Ok(0)
    }

    fn write_vector(&mut self, value: &[u8]) -> errors::Result<usize> {
        Ok(0)
    }
}



struct Packet {}

impl Packet {
    pub fn encode(message_list: MessageList) -> errors::Result<bool> {
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

// ---------------------------------------------------------------------------
//  Tests


#[cfg(test)]
mod tests {
    #[test]
    fn test_data_type() {
        use crate::message::DataType;
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

    #[test]
    fn test_reader() {
        use crate::message::Reader;
        use crate::errors::*;
        
        let buf = vec![10,20,30];
        let mut rdr = Reader::new(&buf[..]);
        let r = rdr.read_i32();

        match r {
            Ok(_) => assert!(false),
            Err(e) => match e {
                Error::Serialization(ee) => match ee {
                    SerializationError::InsufficientSpace() => assert!(true),
                    _ => assert!(false),
                }
                _ => assert!(false),
            }
        };
        assert_eq!(rdr.bytes_read, 0);

        assert_eq!(rdr.read_i8().unwrap(), 10);
        assert_eq!(rdr.read_i8().unwrap(), 20);
        assert_eq!(rdr.read_i8().unwrap(), 30);
        assert_eq!(rdr.bytes_read, 3);

        let i: i32 = 8;
        let buf = vec![8,0,0,0];
        let mut rdr = Reader::new(&buf[..]);
        let ii = rdr.read_i32().unwrap();
        assert_eq!(i, ii);
        assert_eq!(rdr.bytes_read, 4);


        let d: f64 = 3.14159;
        let buf = d.to_le_bytes().to_vec();
        let mut rdr = Reader::new(&buf[..]);
        assert_eq!(d, rdr.read_f64().unwrap());
        assert_eq!(rdr.bytes_read, 8);
        

        let s = String::from("asdf💖");

        let buf = vec![8, 0, 0, 0, 97, 115, 100, 102, 240, 159, 146, 150];
        let mut rdr = Reader::new(&buf[..]);
        let ss = rdr.read_string().unwrap();
        
        assert_eq!(s, ss);
        assert_eq!(rdr.bytes_read, 12);

        let v = vec![97, 115, 100, 102, 240, 159, 146, 150];
        let buf = vec![8, 0, 0, 0, 97, 115, 100, 102, 240, 159, 146, 150];
        let mut rdr = Reader::new(&buf[..]);
        let vv = rdr.read_vector().unwrap();
        assert_eq!(vv.len(), 8);
        assert_eq!(v, vv);
    }

    #[test]
    fn test_writer() {

        use crate::message::Writer;
        use crate::errors::*;

        let mut buffer: Vec<u8> = vec![0; 4];
        let mut writer = Writer::new(&mut buffer);

        assert_eq!(writer.write_i8(-1).unwrap(), 1);
        assert_eq!(writer.write_i8(10).unwrap(), 1);
        assert_eq!(writer.write_i8(-128).unwrap(), 1);
        assert_eq!(writer.write_i8(127).unwrap(), 1);

        assert_eq!(writer.bytes_written, 4);
        // This should fail
        if let Ok(i) = writer.write_i8(-1) {
            assert!(true);
        }
        assert_eq!(writer.bytes_written, 4);

        let b2: Vec<u8> = vec![0b1111_1111, 10, 0b1000_0000, 127];
        assert_eq!(buffer, b2);


        let mut buffer: Vec<u8> = vec![0; 8];
        let mut writer = Writer::new(&mut buffer);

        assert_eq!(writer.write_i32(8).unwrap(), 4);
        assert_eq!(writer.write_i32(-123145).unwrap(), 4);
        assert_eq!(writer.bytes_written, 8);

        if let Ok(i) = writer.write_i32(134) {
            assert!(true);
        }

        assert_eq!(buffer, vec![0x8, 0x0, 0x0, 0x0, 0xF7, 0x1E, 0xFE, 0xFF]);

        let mut buffer: Vec<u8> = vec![0; 16];
        let mut writer = Writer::new(&mut buffer);

        assert_eq!(writer.write_f64(-20391.0).unwrap(), 8);
        assert_eq!(writer.write_f64(2911204.1231).unwrap(), 8);
        assert_eq!(writer.bytes_written, 16);

        if let Ok(i) = writer.write_f64(98723.2342) {
            assert!(true);
        }

        assert_eq!(buffer, vec![0x00, 0x00, 0x00, 0x00, 0xc0, 0xe9, 0xd3, 0xc0,
            0xa5, 0xbd, 0xc1, 0x0f, 0xf2, 0x35, 0x46, 0x41 ]);
    }
}
