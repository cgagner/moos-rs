use crate::message::{Data, Message, ValueType};
use crate::{errors, message::PROTOCOL_CONNECT_MESSAGE};
use crate::{time_local, time_unwarped, time_warped};

use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct AsyncClient {
    stream: Option<TcpStream>,
    write_buffer: Vec<u8>,
    read_buffer: Vec<u8>,
    name: String,
    community: String,
    database_host: String,
    database_port: u16,
}

pub struct InnerAsyncClient {}

impl AsyncClient {
    pub async fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        AsyncClient {
            stream: None,
            write_buffer: vec![0; 200000],
            read_buffer: vec![0; 200000],
            name: name.into(),
            community: "".into(),
            database_host: "localhost".into(),
            database_port: 9000,
        }
    }
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }
    pub fn get_community(&self) -> &str {
        if self.is_connected() {
            self.community.as_str()
        } else {
            ""
        }
    }

    /// Get the name of the client
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn is_time_correction_enabled(&self) -> bool {
        /// TODO
        return true;
    }

    /// Connect to the MOOS database on the specified host and port.
    /// * `host`: hostname or IP address of the MOOS database
    /// * `port`: port of the MOOS database
    pub async fn connect_to<S>(&mut self, host: S, port: u16) -> errors::Result<()>
    where
        S: Into<String>,
    {
        self.database_host = host.into();
        self.database_port = port;
        return self.connect().await;
    }

    /// Connect to the MOOS Database.
    pub async fn connect(&mut self) -> errors::Result<()> {
        if self.is_connected() {
            return Ok(());
        }

        let addr = format!("{}:{}", self.database_host, self.database_port);
        let mut stream = TcpStream::connect(addr).await.unwrap();

        Ok(())
    }
    pub async fn disconnect(&mut self) -> errors::Result<()> {
        if let Some(stream) = &mut self.stream {
            stream
                .shutdown()
                .await
                .expect("Failed to shutdown async_client TcpStream.");
        }
        self.stream = None;

        Ok(())
    }
    pub async fn handshake(&mut self) -> errors::Result<()> {
        if self.is_time_correction_enabled() {
            crate::set_time_skew(0.0);
        }

        // TODO: Need to move these to a constructor
        let mut stream = TcpStream::connect("127.0.0.1:9000").await.unwrap();

        let result = stream
            .write(crate::message::PROTOCOL_CONNECT_MESSAGE.as_bytes())
            .await;

        if let Ok(bytes_written) = result {
            if bytes_written != crate::message::PROTOCOL_CONNECT_MESSAGE.len() {
                return Err(errors::Error::General("Failed to write welcome message"));
            }
        } else {
            return Err(errors::Error::General("Failed to write welcome message"));
        }

        let msg = Message::connect(self.get_name());

        let len = crate::message::encode_slice(msg, &mut self.write_buffer)?;

        let result = stream.write(&mut self.write_buffer[0..len]).await;
        println!("wrote to stream; success={:?}", result);
        println!("Wrote: {:x?}", &self.write_buffer[0..len]);

        let result = stream.read(&mut self.read_buffer).await;

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

            let is_async = match msg.data() {
                Data::String(s) => s == crate::message::ASYNCHRONOUS,
                _ => false,
            };

            // TODO: need to parse hostname=X
            let my_host_name = msg.source_aux();
            // store the hostname

            let skew = msg.double_value();
            if self.is_time_correction_enabled() {
                crate::set_time_skew(skew);
            }
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

        self.stream = Some(stream);

        Ok(())
    }
}
