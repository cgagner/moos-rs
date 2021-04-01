use crate::errors;
use crate::message::{Data, Message, ValueType};
use crate::{time_local, time_unwarped, time_warped};

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct AsyncClient {
    stream: TcpStream,
    write_buffer: Vec<u8>,
    read_buffer: Vec<u8>,
    community: String,
}

pub struct InnerAsyncClient {}

impl AsyncClient {
    pub async fn new() -> Self {
        AsyncClient {
            stream: TcpStream::connect("127.0.0.1:9000").await.unwrap(),
            write_buffer: vec![0; 200000],
            read_buffer: vec![0; 200000],
            community: "".into(),
        }
    }

    pub fn get_community(&self) -> &str {
        self.community.as_str()
    }

    pub fn is_time_correction_enabled(&self) -> bool {
        /// TODO
        return true;
    }

    pub async fn handshake(&mut self) -> errors::Result<()> {
        if self.is_time_correction_enabled() {
            crate::set_time_skew(0.0);
        }

        // TODO: Need to move these to a constructor
        self.stream = TcpStream::connect("127.0.0.1:9000").await.unwrap();

        let result = self
            .stream
            .write(crate::message::PROTOCOL_CONNECT_MESSAGE.as_bytes())
            .await;

        if let Ok(bytes_written) = result {
            if bytes_written != crate::message::PROTOCOL_CONNECT_MESSAGE.len() {
                return Err(errors::Error::General("Failed to write welcome message"));
            }
        } else {
            return Err(errors::Error::General("Failed to write welcome message"));
        }

        let msg = Message::connect("umm-1");

        let len = crate::message::encode_slice(msg, &mut self.write_buffer)?;

        let result = self.stream.write(&mut self.write_buffer[0..len]).await;
        println!("wrote to stream; success={:?}", result);
        println!("Wrote: {:x?}", &self.write_buffer[0..len]);

        let result = self.stream.read(&mut self.read_buffer).await;

        if let Ok(size) = result {
            println!("Read: {}", size);
        } else {
            println!("Error: {:?} ", result);
        }

        let (msg_list, bytes_read) = if let Ok(bytes_read) = result {
            crate::message::decode_slice(&self.read_buffer)?
        } else {
            return Err(errors::Error::General("Failed to decode welcome message."));
        };

        for msg in msg_list {
            if let crate::message::MessageType::Poision = msg.message_type {
                return Err(errors::Error::General("Client poisioned during handshake."));
            }
            self.community = msg.originating_community().into();
            // Check for asynchronous
            // store the hostname
        }

        // println!("Bytes read: {}", bytes_read);
        // println!("Number of messages: {}", msg_list.len());
        // for msg in msg_list {
        //     println!("MessageType: {:?} ", msg.data_type());
        //     println!("Source: {}", msg.source());
        //     println!("SourceAux: {}", msg.source_aux());
        //     println!("Community: {}", msg.originating_community());
        //     match msg.value() {
        //         ValueType::Binary(b) => println!("Binary: {:x?}", b),
        //         ValueType::String(s) => println!("String: {}", s),
        //         ValueType::Double(d) => println!("Double: {}", d),
        //     };

        //     match msg.data() {
        //         Data::String(s) => println!("String: {}", s),
        //         Data::Binary(b) => println!("Binary: {:x?}", b),
        //     }
        // }

        Ok(())
    }
}
