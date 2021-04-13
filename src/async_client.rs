use crate::{errors, message::PROTOCOL_CONNECT_MESSAGE};
use crate::{
    errors::Result,
    message::{Data, Message, MessageType, ValueType},
};
use crate::{time_local, time_unwarped, time_warped};

use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::sleep,
};

pub struct AsyncClient {
    stream: Option<TcpStream>,
    write_buffer: Vec<u8>,
    read_buffer: Vec<u8>,
    name: String,
    community: String,
    database_host: String,
    database_port: u16,
    last_connected_time: f64,
    current_id: i32,
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
            last_connected_time: 0.0,
            current_id: 0,
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

        let mut attempt: i32 = 0;

        while let Err(_) = TcpStream::connect(addr.clone()).await {
            println!(
                "AsyncClient failed to connect to {} after {} attempts.",
                addr, attempt
            );
            attempt += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }

        self.stream = Some(TcpStream::connect(addr).await.unwrap());

        if let Err(e) = self.handshake().await {
            return Err(e);
        }

        self.last_connected_time = time_warped();

        // TODO: Need to call the on_connect callback

        Ok(())
    }

    /// Disconnect from the MOOSDB.
    pub async fn disconnect(&mut self) -> errors::Result<()> {
        if let Some(stream) = &mut self.stream {
            stream
                .shutdown()
                .await
                .expect("Failed to shutdown async_client TcpStream.");
        }
        self.stream = None;

        // TODO: Need to call the on_disconnect callback
        Ok(())
    }

    pub async fn subscribe(&mut self, variable: &str, interval: f64) -> errors::Result<()> {
        if !self.is_connected() {
            return Err(errors::Error::General(
                "AsyncClient::subscribe: cannot subscribe because the client is not connected.",
            ));
        }

        let mut message = Message::register(self.get_name(), variable, interval);

        // TODO: Need to create a helper method or a builder to handle setting
        // the other fields of the message from the client. This includes
        // the client name, timestamp, incrementing the id.

        return self.send_message(&mut message).await;
    }

    /// Send a message to the MOOSDB.
    ///
    /// # Arguments
    /// * `message`: Message to be sent
    async fn send_message<'m>(&mut self, message: &mut Message<'m>) -> errors::Result<()> {
        if !self.is_connected() {
            return Err(errors::Error::General(
                "AsyncClient::send_message: failed to send because the client is not connected.",
            ));
        }
        message.source = String::from(self.get_name());

        message.id = match message.message_type() {
            MessageType::ServerRequest => -2,
            _ => self.current_id,
        };

        self.current_id += 1;

        // TODO: Look at switching this to use a channel

        let len = crate::message::encode_slice(&message, &mut self.write_buffer)?;

        if let Some(stream) = &mut self.stream {
            let result = stream.write(&mut self.write_buffer[0..len]).await;
            if let Err(_) = result {
                return Err(errors::Error::General(
                    "AsyncClient::send_message: failed to send message",
                ));
            }
            return Ok(());
        } else {
            return Err(errors::Error::General(
                "AsyncClient::send_message: failed to send because the client is not connected.",
            ));
        }
    }

    /// Handle the handshake with the MOOSDB to initiate the communication.
    /// The handshake should happen after the socket is conencted to the MOOSDB.
    /// Attempting to handshake without being connected will result in an error
    /// being returned.
    async fn handshake(&mut self) -> errors::Result<()> {
        let time_correction_enabled = self.is_time_correction_enabled();
        let client_name = String::from(self.get_name());

        if time_correction_enabled {
            crate::set_time_skew(0.0);
        }

        let stream = if let Some(stream) = &mut self.stream {
            stream
        } else {
            return Err(errors::Error::General(
                "AsyncClient::handshake faile to get the TcpStream",
            ));
        };

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

        let msg = Message::connect(client_name.as_str());

        let len = crate::message::encode_slice(&msg, &mut self.write_buffer)?;

        let result = stream.write(&mut self.write_buffer[0..len]).await;
        println!("wrote to stream; success={:?}", result);
        println!("Wrote: {:x?}", &self.write_buffer[0..len]);

        let result = stream.read(&mut self.read_buffer).await;

        if let Ok(size) = result {
            println!("Read: {}", size);
        } else {
            println!("Error: {:?} ", result);
        }

        let (msg_list, _) = if let Ok(_) = result {
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
            if time_correction_enabled {
                crate::set_time_skew(skew);
            }
        }

        Ok(())
    }

    // TODO: Change this to be private
    pub async fn read_loop(&mut self) -> errors::Result<()> {
        loop {
            let stream = if let Some(stream) = &mut self.stream {
                stream
            } else {
                return Err(errors::Error::General(
                    "AsyncClient::handshake faile to get the TcpStream",
                ));
            };
            let (reader, _writer) = &mut stream.split();

            let result = reader.read(&mut self.read_buffer).await;

            let (msg_list, _) = if let Ok(bytes_read) = result {
                if bytes_read > 0 {
                    crate::message::decode_slice(&self.read_buffer)?
                } else {
                    continue;
                }
            } else {
                return Err(errors::Error::General("Failed to decode message."));
            };

            println!("Received {} messages.", msg_list.len());
            for message in msg_list {
                println!("Received Message of type: {}", message);
            }
        }
    }
}
