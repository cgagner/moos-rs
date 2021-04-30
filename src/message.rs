// message.rs
//

use super::errors;
use crate::errors::{InsufficientSpaceError, Result};
use crate::{time_local, time_unwarped, time_warped};
use core::convert::TryInto;
use core::mem;
use std::{collections::btree_map::Values, fmt, fmt::Display, io::Error, str::from_utf8};

pub const PROTOCOL_CONNECT_MESSAGE: &str = "ELKS CAN'T DANCE 2/8/10\0\0\0\0\0\0\0\0\0";
pub const ASYNCHRONOUS: &str = "asynchronous";

pub enum Data {
    String(String),
    Binary(Vec<u8>),
}

pub struct Message {
    pub(crate) id: i32,
    pub(crate) message_type: MessageType,
    pub(crate) data_type: DataType,
    pub(crate) double_value: f64,
    /// Auxiliary Double Value - Only used by `NOTIFY` and `TIMING` messages.
    /// Should not be exposed though API.
    pub(crate) double_value2: f64,
    pub(crate) data: Data,
    pub(crate) key: String,
    pub(crate) time: f64,
    pub(crate) source: String,
    pub(crate) source_aux: String,
    pub(crate) originating_community: String,
}

pub type MessageList = Vec<Message>;

impl Message {
    /// Create a message that has a string value.
    /// * `key`: Key of the message
    /// * `value`: Value to put in the message
    pub fn from_string<S>(key: S, value: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            id: 0,
            message_type: MessageType::Data,
            data_type: DataType::String,
            double_value: 0.0,
            double_value2: 0.0,
            data: Data::String(value.into()),
            time: time_warped(),
            key: key.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn connect(client_name: &str) -> Self {
        Message {
            id: -1,                          //
            message_type: MessageType::Data, //
            data_type: DataType::String,
            double_value: -1.0,
            double_value2: -1.0,
            data: Data::String(client_name.into()),
            time: time_warped(),
            key: ASYNCHRONOUS.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn timing(client_name: &str) -> Self {
        Message {
            id: -1,                            //
            message_type: MessageType::Timing, //
            data_type: DataType::Double,
            double_value: 0.0,
            double_value2: -1.0,
            data: Data::String(client_name.into()),
            time: time_warped(),
            key: "_async_timing".into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn register(client_name: &str, key: &str, interval: f64) -> Self {
        Message {
            id: -1,                              //
            message_type: MessageType::Register, //
            data_type: DataType::Double,
            double_value: interval,
            double_value2: -1.0,
            data: Data::String(client_name.into()),
            time: time_warped(),
            key: key.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn wildcard_register(client_name: &str, value: &str) -> Self {
        Message {
            id: -1,                                      //
            message_type: MessageType::WildcardRegister, //
            data_type: DataType::String,
            double_value: -1.0,
            double_value2: -1.0,
            data: Data::String(value.into()),
            time: time_warped(),
            key: client_name.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn unregister(client_name: &str, key: &str, interval: f64) -> Self {
        Message {
            id: -1,                                //
            message_type: MessageType::Unregister, //
            data_type: DataType::Double,
            double_value: interval,
            double_value2: -1.0,
            data: Data::String(client_name.into()),
            time: time_warped(),
            key: key.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn wildcard_unregister(client_name: &str, value: &str) -> Self {
        Message {
            id: -1,                                        //
            message_type: MessageType::WildcardUnregister, //
            data_type: DataType::String,
            double_value: -1.0,
            double_value2: -1.0,
            data: Data::String(value.into()),
            time: time_warped(),
            key: client_name.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    /// Create a new message
    pub(crate) fn new<S>(message_type: MessageType, key: S) -> Self
    where
        S: Into<String>,
    {
        Message {
            id: 0,                      //
            message_type: message_type, //
            data_type: DataType::Double,
            double_value: 0.0,
            double_value2: 0.0,
            data: Data::Binary(Vec::new()),
            time: 0.0,
            key: key.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn notify_double(key: &str, value: f64, time: f64) -> Self {
        Message {
            id: 0,                             //
            message_type: MessageType::Notify, //
            data_type: DataType::Double,
            double_value: value,
            double_value2: 0.0,
            data: Data::Binary(Vec::new()),
            time: time,
            key: key.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn notify_data(key: &str, value: &Vec<u8>, time: f64) -> Self {
        Message {
            id: 0,                             //
            message_type: MessageType::Notify, //
            data_type: DataType::Binary,
            double_value: 0.0,
            double_value2: 0.0,
            data: Data::Binary(value.clone()),
            time: time,
            key: key.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub(crate) fn notify_string<S>(key: &str, value: S, time: f64) -> Self
    where
        S: Into<String>,
    {
        Message {
            id: 0,                             //
            message_type: MessageType::Notify, //
            data_type: DataType::String,
            double_value: 0.0,
            double_value2: 0.0,
            data: Data::String(value.into()),
            time: time,
            key: key.into(),
            source: String::new(),
            source_aux: String::new(),
            originating_community: String::new(),
        }
    }

    pub fn data(&self) -> &Data {
        &self.data
    }

    /// Double value - Normally, you should use the data. However, some
    /// of the internal messages use both the double value and the string
    /// values.
    pub(crate) fn double_value(&self) -> f64 {
        self.double_value
    }

    /// Type of the data
    pub fn data_type(&self) -> DataType {
        self.data_type
    }

    /// Type of the message
    pub(crate) fn message_type(&self) -> MessageType {
        self.message_type
    }

    pub(crate) fn is_notify(&self) -> bool {
        if let MessageType::Notify = self.message_type {
            true
        } else {
            false
        }
    }

    /// Key of the message.
    pub fn key(&self) -> &str {
        self.key.as_str()
    }

    /// Time of the message
    pub fn time(&self) -> f64 {
        self.time
    }

    /// Source of the message
    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    /// Extra source information
    pub fn source_aux(&self) -> &str {
        self.source_aux.as_str()
    }

    /// Originiating community name
    pub fn originating_community(&self) -> &str {
        self.originating_community.as_str()
    }

    /// Data Value in the message. This is either a double,
    /// &str, or a &[[u8]].
    ///
    /// # Example moos::message::Message
    /// ```
    /// use moos::message::{Message, ValueType};
    /// let mut m: Message = Message::from_string("DEPLOY", "true");
    /// if let ValueType::String(s) = m.value() {
    ///    assert_eq!(s, "true");
    /// }
    /// ```
    pub fn value(&self) -> ValueType {
        match self.data_type {
            DataType::Double => ValueType::Double(self.double_value),
            _ => match &self.data {
                Data::Binary(b) => ValueType::Binary(b.as_slice()),
                Data::String(s) => ValueType::String(s.as_str()),
            },
        }
    }

    /// Decode a [Message] from a [[u8]] slice
    ///
    /// Returns [Result] with a tupple of a [Message] and a [usize] if successful.
    /// If there was a problem decoding a message, an error will be returned.
    /// In most cases, this is because there is insufficient space or because
    /// a string is not a valid UTF8 string.
    pub fn decode_slice(buffer: &[u8]) -> errors::Result<(Message, usize)> {
        let mut reader = Reader::new(buffer);

        let length = reader.read_i32()? as usize;
        if buffer.len() + core::mem::size_of::<i32>() < length {
            return Err(errors::InsufficientSpaceError);
        }

        let id = reader.read_i32()?;
        let message_type = MessageType::from_byte(reader.read_i8()?);
        let data_type = DataType::from_byte(reader.read_i8()?);
        let source = reader.read_string()?;
        let source_aux = reader.read_string()?;
        let originating_community = reader.read_string()?;
        let key = reader.read_string()?;
        let time = reader.read_f64()?;
        let double_value = reader.read_f64()?;
        let double_value2 = reader.read_f64()?;

        let data = match data_type {
            DataType::String => Data::String(reader.read_string()?),
            _ => Data::Binary(reader.read_vector()?),
        };

        let msg = Message {
            id,
            message_type,
            data_type,
            double_value,
            double_value2,
            data,
            time,
            key,
            source,
            source_aux,
            originating_community,
        };

        Ok((msg, reader.bytes_read))
    }

    /// Encode a [Message] into a [[u8]] slice.
    ///
    /// Returns a [Result] with the [usize] number of bytes written into the
    /// slice if successful. Otherwise, returns a error. In most cases, this
    /// is because there is insufficient space or because a string is not
    /// a valid UTF8 string.
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
        match &self.data {
            Data::Binary(b) => writer.write_vector(&b)?,
            Data::String(s) => writer.write_string(&s)?,
        };

        Ok(writer.bytes_written)
    }

    /// Returns the size of the message when serialized
    pub fn get_size(&self) -> i32 {
        (
            //
            mem::size_of::<i32>() // Length
             + mem::size_of_val(&self.id) // ID
             + mem::size_of::<i8>() // message type 
             + mem::size_of::<i8>() // data type 
             + mem::size_of::<i32>() + self.source.len()
             + mem::size_of::<i32>() + self.source_aux.len()
             + mem::size_of::<i32>() + self.originating_community.len()
             + mem::size_of::<i32>() + self.key.len()
             + mem::size_of_val(&self.time)
             + mem::size_of_val(&self.double_value)
             + mem::size_of_val(&self.double_value2)
             + mem::size_of::<i32>() + match &self.data {
                Data::Binary(b) => b.len(),
                Data::String(s) => s.len(),
            }
        ) as i32
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(type: {:?}", self.message_type())?;
        write!(f, ",data_type: {:?}", self.data_type())?;
        write!(f, ",key: {}", self.key())?;
        match self.value() {
            ValueType::Double(d) => write!(f, ",value: {}", d)?,
            ValueType::String(s) => write!(f, ",value: {}", s)?,
            ValueType::Binary(b) => write!(f, ",value: {:?}", b)?,
            _ => write!(f, ",value: null")?,
        };
        write!(f, ")")
    }
}

pub fn encode_slice(msg: &Message, buffer: &mut [u8]) -> errors::Result<usize> {
    const PACKET_HEADER_SIZE: usize = core::mem::size_of::<i32>() * 2 + core::mem::size_of::<i8>();
    let mut writer = Writer::new(buffer);
    writer.write_i32(msg.get_size() + PACKET_HEADER_SIZE as i32)?; // number of bytes
    writer.write_i32(1)?; // Number of messages
    writer.write_i8(0)?; // Compression enabled
    let len = writer.bytes_written;
    let len = len + msg.encode_slice(&mut buffer[PACKET_HEADER_SIZE..])?;
    Ok(len)
}

pub fn decode_slice(buffer: &[u8]) -> errors::Result<(MessageList, usize)> {
    let mut reader = Reader::new(buffer);
    let total_bytes = reader.read_i32()?;
    if buffer.len() < total_bytes as usize {
        return Err(errors::InsufficientSpaceError);
    }

    let expected_messages = reader.read_i32()?;
    reader.read_i8()?; // compression enabled - not used

    let mut bytes_read = reader.bytes_read;
    drop(reader);
    let mut msg_list = MessageList::new();
    for i in 0..expected_messages {
        let (msg, message_size) = Message::decode_slice(&buffer[bytes_read..])?;
        bytes_read += message_size;
        msg_list.push(msg);
    }

    Ok((msg_list, bytes_read))
}

/// Type of the message.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub(crate) enum MessageType {
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

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
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

pub enum ValueType<'a> {
    Double(f64),
    String(&'a str),
    Binary(&'a [u8]),
}

struct Reader<'a> {
    bytes_read: usize,
    buffer: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Create a new reader for a byte buffer
    fn new(buffer: &'a [u8]) -> Self {
        Reader {
            bytes_read: 0,
            buffer,
        }
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
        let buf: &[u8; core::mem::size_of::<i32>()] = match self.buffer
            [self.bytes_read..(self.bytes_read + core::mem::size_of::<i32>())]
            .try_into()
        {
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
        let buf: &[u8; core::mem::size_of::<f64>()] = match self.buffer
            [self.bytes_read..(self.bytes_read + core::mem::size_of::<f64>())]
            .try_into()
        {
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
        let s = match std::str::from_utf8(&self.buffer[self.bytes_read..(self.bytes_read + length)])
        {
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

impl<'a> Writer<'a> {
    fn new(buffer: &'a mut [u8]) -> Self {
        Writer {
            bytes_written: 0,
            buffer,
        }
    }

    fn write_i8(&mut self, value: i8) -> errors::Result<usize> {
        if self.bytes_written + core::mem::size_of::<i8>() > self.buffer.len() {
            return Err(errors::InsufficientSpaceError);
        }
        self.buffer[self.bytes_written] = value as u8;
        self.bytes_written += core::mem::size_of::<i8>();
        Ok(core::mem::size_of::<i8>())
    }

    fn write_i32(&mut self, value: i32) -> errors::Result<usize> {
        if self.bytes_written + core::mem::size_of::<i32>() > self.buffer.len() {
            return Err(errors::InsufficientSpaceError);
        }
        self.buffer[self.bytes_written..(self.bytes_written + core::mem::size_of::<i32>())]
            .copy_from_slice(&value.to_le_bytes());
        self.bytes_written += core::mem::size_of::<i32>();
        Ok(core::mem::size_of::<i32>())
    }

    fn write_f64(&mut self, value: f64) -> errors::Result<usize> {
        if self.bytes_written + core::mem::size_of::<f64>() > self.buffer.len() {
            return Err(errors::InsufficientSpaceError);
        }
        self.buffer[self.bytes_written..(self.bytes_written + core::mem::size_of::<f64>())]
            .copy_from_slice(&value.to_le_bytes());
        self.bytes_written += core::mem::size_of::<f64>();
        Ok(core::mem::size_of::<f64>())
    }

    #[inline]
    fn write_string(&mut self, value: &str) -> errors::Result<usize> {
        return self.write_vector(value.as_bytes());
    }

    fn write_vector(&mut self, value: &[u8]) -> errors::Result<usize> {
        if self.bytes_written + value.len() + core::mem::size_of::<i32>() > self.buffer.len() {
            return Err(errors::InsufficientSpaceError);
        }
        self.write_i32(value.len() as i32)?;
        self.buffer[self.bytes_written..(self.bytes_written + value.len())].copy_from_slice(value);
        self.bytes_written += value.len();
        Ok(value.len() + core::mem::size_of::<i32>())
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
    use log::debug;

    use crate::errors::*;
    use crate::message::Reader;
    use crate::message::Writer;
    use crate::message::{Message, ValueType};

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
    fn test_reader_i8() {
        let buf = vec![10, 20, 30];
        let mut rdr = Reader::new(&buf[..]);
        let r = rdr.read_i32();

        match r {
            Ok(_) => assert!(false),
            Err(e) => match e {
                Error::Serialization(ee) => match ee {
                    SerializationError::InsufficientSpace() => assert!(true),
                    _ => assert!(false),
                },
                _ => assert!(false),
            },
        };
        assert_eq!(rdr.bytes_read, 0);

        assert_eq!(rdr.read_i8().unwrap(), 10);
        assert_eq!(rdr.read_i8().unwrap(), 20);
        assert_eq!(rdr.read_i8().unwrap(), 30);
        assert_eq!(rdr.bytes_read, 3);
    }
    #[test]
    fn test_reader_i32() {
        let i: i32 = 8;
        let buf = vec![8, 0, 0, 0];
        let mut rdr = Reader::new(&buf[..]);
        let ii = rdr.read_i32().unwrap();
        assert_eq!(i, ii);
        assert_eq!(rdr.bytes_read, 4);
    }

    #[test]
    fn test_reader_f64() {
        let d: f64 = 3.14159;
        let buf = d.to_le_bytes().to_vec();
        let mut rdr = Reader::new(&buf[..]);
        assert_eq!(d, rdr.read_f64().unwrap());
        assert_eq!(rdr.bytes_read, 8);
    }

    #[test]
    fn test_reader_string() {
        let s = String::from("asdf💖");

        let buf = vec![8, 0, 0, 0, 97, 115, 100, 102, 240, 159, 146, 150];
        let mut rdr = Reader::new(&buf[..]);
        let ss = rdr.read_string().unwrap();

        assert_eq!(s, ss);
        assert_eq!(rdr.bytes_read, 12);
    }
    #[test]
    fn test_reader_vector() {
        let v = vec![97, 115, 100, 102, 240, 159, 146, 150];
        let buf = vec![8, 0, 0, 0, 97, 115, 100, 102, 240, 159, 146, 150];
        let mut rdr = Reader::new(&buf[..]);
        let vv = rdr.read_vector().unwrap();
        assert_eq!(vv.len(), 8);
        assert_eq!(v, vv);
    }

    #[test]
    fn test_writer_i8() {
        let mut buffer: Vec<u8> = vec![0; 4];
        let mut writer = Writer::new(&mut buffer);

        assert_eq!(writer.write_i8(-1).unwrap(), 1);
        assert_eq!(writer.write_i8(10).unwrap(), 1);
        assert_eq!(writer.write_i8(-128).unwrap(), 1);
        assert_eq!(writer.write_i8(127).unwrap(), 1);

        assert_eq!(writer.bytes_written, 4);
        // This should fail
        if let Ok(i) = writer.write_i8(-1) {
            assert!(false);
        } else {
            assert!(true);
        }
        assert_eq!(writer.bytes_written, 4);

        let b2: Vec<u8> = vec![0b1111_1111, 10, 0b1000_0000, 127];
        assert_eq!(buffer, b2);
    }

    #[test]
    fn test_writer_i32() {
        let mut buffer: Vec<u8> = vec![0; 8];
        let mut writer = Writer::new(&mut buffer);

        assert_eq!(writer.write_i32(8).unwrap(), 4);
        assert_eq!(writer.write_i32(-123145).unwrap(), 4);
        assert_eq!(writer.bytes_written, 8);

        if let Ok(i) = writer.write_i32(134) {
            assert!(false);
        } else {
            assert!(true);
        }

        assert_eq!(buffer, vec![0x8, 0x0, 0x0, 0x0, 0xF7, 0x1E, 0xFE, 0xFF]);
    }

    #[test]
    fn test_writer_f64() {
        let mut buffer: Vec<u8> = vec![0; 16];
        let mut writer = Writer::new(&mut buffer);

        assert_eq!(writer.write_f64(-20391.0).unwrap(), 8);
        assert_eq!(writer.write_f64(2911204.1231).unwrap(), 8);
        assert_eq!(writer.bytes_written, 16);

        if let Ok(i) = writer.write_f64(98723.2342) {
            assert!(false);
        } else {
            assert!(true);
        }

        assert_eq!(
            buffer,
            vec![
                0x00, 0x00, 0x00, 0x00, 0xc0, 0xe9, 0xd3, 0xc0, 0xa5, 0xbd, 0xc1, 0x0f, 0xf2, 0x35,
                0x46, 0x41
            ]
        );
    }

    #[test]
    fn test_writer_string() {
        let mut buffer: Vec<u8> = vec![0; 12];
        let mut writer = Writer::new(&mut buffer);
        let s = String::from("asdf💖");
        assert_eq!(writer.write_string(&s).unwrap(), 12);
        if let Ok(i) = writer.write_string("this should fail") {
            assert!(false);
        } else {
            assert!(true);
        }
        assert_eq!(writer.bytes_written, 12);
        assert_eq!(
            buffer,
            vec![8, 0, 0, 0, 97, 115, 100, 102, 240, 159, 146, 150]
        );
    }

    #[test]
    fn test_writer_vector() {
        let mut buffer: Vec<u8> = vec![0; 12];
        let mut writer = Writer::new(&mut buffer);
        let v: Vec<u8> = vec![97, 115, 100, 102, 240, 159, 146, 150];

        assert_eq!(writer.write_vector(&v).unwrap(), 12);
        if let Ok(i) = writer.write_vector(&v) {
            assert!(false);
        } else {
            assert!(true);
        }
        assert_eq!(writer.bytes_written, 12);
        assert_eq!(
            buffer,
            vec![8, 0, 0, 0, 97, 115, 100, 102, 240, 159, 146, 150]
        );
    }

    #[test]
    fn test_message() {
        let mut m: Message = Message::from_string("DEPLOY", "true");
        assert_eq!(m.key(), "DEPLOY");

        println!("Key: {}", m.key());

        match m.value() {
            ValueType::Binary(b) => println!("Binary: {:?}", b),
            ValueType::String(s) => println!("String: {:?}", s),
            ValueType::Double(d) => println!("Double: {}", d),
        };

        if let ValueType::String(s) = m.value() {
            assert_eq!(s, "true");
        } else {
            assert!(false);
        }
    }

    // Timestamp: 1616542133 +- 10 mintues

    // From client to MOOSDB
    // "ELKS CAN'T DANCE 2/8/10".
    // 45 4c 4b 53 20 43 41 4e 27 54 20 44 41 4e 43 45 20 32 2f 38 2f 31 30 00 00 00 00 00 00 00 00 00

    // From client to MOOSDB
    // Initial connect message - 80 bytes
    // 50 00 00 00 01 00 00 00 00 47 00 00 00 ff ff ff
    // ff 69 53 00 00 00 00 00 00 00 00 00 00 00 00 0c
    // 00 00 00 61 73 79 6e 63 68 72 6f 6e 6f 75 73 38
    // c0 a2 de 9c 16 d8 41 00 00 00 00 00 00 f0 bf 00
    // 00 00 00 00 00 f0 bf 05 00 00 00 75 6d 6d 2d 31

    // 50 00 00 00  - Packet header size: 0x50
    // 01 00 00 00  - Number of messages: 1
    // 00           - Compression: 0
    // 47 00 00 00  - Message size: 0x47
    // ff ff ff ff  - Id: -1
    // 69           - Message Type: Data
    // 53           - Data Type: String
    // 00 00 00 00  - source
    // 00 00 00 00  - source_aux
    // 00 00 00 00  - community
    // 0c 00 00 00  - Key size - 12: asynchronous
    // 61 73 79 6e 63 68 72 6f 6e 6f 75 73
    // 38 c0 a2 de 9c 16 d8 41 - Time: 1616540538.542982
    // 00 00 00 00 00 00 f0 bf - Double value: -1
    // 00 00 00 00 00 00 f0 bf - Double value2: -1
    // 05 00 00 00 - String value size - 5
    // 75 6d 6d 2d 31 - umm-1

    // From MOOSDB to client
    // Welcome Message - 113 bytes
    // 71 00 00 00 01 00 00 00 00 68 00 00 00 ff ff ff
    // ff 57 44 00 00 00 00 24 00 00 00 68 6f 73 74 6e
    // 61 6d 65 3d 43 68 72 69 73 74 6f 70 68 65 72 73
    // 4d 42 50 2e 76 65 72 69 7a 6f 6e 2e 6e 65 74 02
    // 00 00 00 23 31 00 00 00 00 4a 28 a9 de 9c 16 d8
    // 41 00 00 00 00 38 a0 b9 3f 00 00 00 00 00 00 f0
    // bf 0c 00 00 00 61 73 79 6e 63 68 72 6f 6e 6f 75
    // 73

    // From client to MOOSDB
    // Timing message - 76 bytes
    // 4c 00 00 00 01 00 00 00 00 43 00 00 00 ff ff ff
    // ff 54 44 00 00 00 00 00 00 00 00 00 00 00 00 0d
    // 00 00 00 5f 61 73 79 6e 63 5f 74 69 6d 69 6e 67
    // 26 c7 be de 9c 16 d8 41 00 00 00 00 00 00 00 00
    // 00 00 00 00 00 00 f0 bf 00 00 00 00

    // From MOOSDB to client
    // Timing response - 76 bytes
    // 4c 00 00 00 01 00 00 00 00 43 00 00 00 ff ff ff
    // ff 54 44 00 00 00 00 00 00 00 00 00 00 00 00 0d
    // 00 00 00 5f 61 73 79 6e 63 5f 74 69 6d 69 6e 67
    // 26 c7 be de 9c 16 d8 41 4f cb be de 9c 16 d8 41
    // 00 00 00 00 00 00 00 00 00 00 00 00

    // From client to MOOSDB
    // Data message? - 205 bytes
    // cd 00 00 00 01 00 00 00 00 c4 00 00 00 00 00 00
    // 00 4e 53 05 00 00 00 75 6d 6d 2d 31 00 00 00 00
    // 00 00 00 00 0c 00 00 00 55 4d 4d 2d 31 5f 53 54
    // 41 54 55 53 33 53 cd de 9c 16 d8 41 00 00 00 00
    // 00 00 f0 bf 00 00 00 00 00 00 f0 bf 7d 00 00 00
    // 41 70 70 45 72 72 6f 72 46 6c 61 67 3d 66 61 6c
    // 73 65 2c 55 70 74 69 6d 65 3d 30 2e 36 36 38 36
    // 2c 63 70 75 6c 6f 61 64 3d 30 2e 36 32 36 35 2c
    // 6d 65 6d 6f 72 79 5f 6b 62 3d 31 31 39 32 2c 6d
    // 65 6d 6f 72 79 5f 6d 61 78 5f 6b 62 3d 31 31 39
    // 32 2c 4d 4f 4f 53 4e 61 6d 65 3d 75 6d 6d 2d 31
    // 2c 50 75 62 6c 69 73 68 69 6e 67 3d 22 22 2c 53
    // 75 62 73 63 72 69 62 69 6e 67 3d 22 22

    // From client to MOOSDB
    // Timing - 76 bytes
    // 4c 00 00 00 01 00 00 00 00 43 00 00 00 ff ff ff
    // ff 54 44 00 00 00 00 00 00 00 00 00 00 00 00 0d
    // 00 00 00 5f 61 73 79 6e 63 5f 74 69 6d 69 6e 67
    // 16 f8 0d df 9c 16 d8 41 00 00 00 00 00 00 00 00
    // 00 00 00 00 00 00 f0 bf 00 00 00 00

    // From MOOSDB to Client
    // Timing response - 76 bytes
    // 4c 00 00 00 01 00 00 00 00 43 00 00 00 ff ff ff
    // ff 54 44 00 00 00 00 00 00 00 00 00 00 00 00 0d
    // 00 00 00 5f 61 73 79 6e 63 5f 74 69 6d 69 6e 67
    // 16 f8 0d df 9c 16 d8 41 ec fb 0d df 9c 16 d8 41
    // 00 00 00 00 00 00 00 00 00 00 00 00

    // From client to MOOSDB
    // Data? -
    // 0000   db 00 00 00 01 00 00 00 00 d2 00 00 00 01 00 00
    // 00 4e 53 05 00 00 00 75 6d 6d 2d 31 00 00 00 00
    // 00 00 00 00 0c 00 00 00 55 4d 4d 2d 31 5f 53 54
    // 41 54 55 53 c6 df 4d df 9c 16 d8 41 00 00 00 00
    // 00 00 f0 bf 00 00 00 00 00 00 f0 bf 8b 00 00 00
    // 41 70 70 45 72 72 6f 72 46 6c 61 67 3d 66 61 6c
    // 73 65 2c 55 70 74 69 6d 65 3d 32 2e 36 37 37 31
    // 38 2c 63 70 75 6c 6f 61 64 3d 30 2e 39 35 36 36
    // 2c 6d 65 6d 6f 72 79 5f 6b 62 3d 31 32 33 32 2c
    // 6d 65 6d 6f 72 79 5f 6d 61 78 5f 6b 62 3d 31 32
    // 33 32 2c 4d 4f 4f 53 4e 61 6d 65 3d 75 6d 6d 2d
    // 31 2c 50 75 62 6c 69 73 68 69 6e 67 3d 22 55 4d
    // 4d 2d 31 5f 53 54 41 54 55 53 2c 22 2c 53 75 62
    // 73 63 72 69 62 69 6e 67 3d 22 22
}
