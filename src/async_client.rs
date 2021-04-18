use crate::{
    errors,
    message::{DataType, PROTOCOL_CONNECT_MESSAGE},
};
use crate::{
    errors::Result,
    message::{Data, Message, MessageType, ValueType},
};
use crate::{time_local, time_unwarped, time_warped};

use log::{debug, error, info, trace, warn};
use std::net::{Shutdown, SocketAddr};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf},
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
    outbox: Option<tokio::sync::mpsc::UnboundedSender<Message>>,
}

pub trait Publish<D> {
    fn publish(&mut self, key: &str, value: D) -> errors::Result<()>;
}

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
            outbox: None,
        }
    }
    pub fn is_connected(&self) -> bool {
        self.outbox.is_some()
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

        let mut result = TcpStream::connect(addr.clone()).await;
        while let Err(_) = result {
            trace!(
                "AsyncClient failed to connect to {} after {} attempts.",
                addr,
                attempt
            );
            attempt += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            result = TcpStream::connect(addr.clone()).await;
        }

        let mut stream = match result {
            Ok(stream) => stream,
            Err(e) => {
                return Err(errors::Error::General(
                    "AsyncClient somehow got an invalid stream while connecting.",
                ))
            }
        };

        if let Err(e) = self.handshake(&mut stream).await {
            return Err(e);
        }

        self.last_connected_time = time_warped();

        // TODO: Need to call the on_connect callback

        let (mut reader, mut writer) = tokio::io::split(stream);

        let (outbox, rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

        let reader_outbox = outbox.clone();
        tokio::spawn(async move {
            AsyncClient::read_loop(reader, reader_outbox).await;
        });

        tokio::spawn(async move { AsyncClient::write_loop(writer, rx).await });

        self.outbox = Some(outbox.clone());
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

    pub fn subscribe(&mut self, key: &str, interval: f64) -> errors::Result<()> {
        if !self.is_connected() {
            return Err(errors::Error::General(
                "AsyncClient::subscribe: cannot subscribe because the client is not connected.",
            ));
        }

        let mut message = Message::register(self.get_name(), key, interval);

        // TODO: Need to create a helper method or a builder to handle setting
        // the other fields of the message from the client. This includes
        // the client name, timestamp, incrementing the id.

        return self.send_message(message);
    }

    /// Send a message to the MOOSDB.
    ///
    /// # Arguments
    /// * `message`: Message to be sent
    fn send_message(&mut self, mut message: Message) -> errors::Result<()> {
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

        if let Some(outbox) = &mut self.outbox {
            let result = outbox.send(message);
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
    async fn handshake(&mut self, stream: &mut TcpStream) -> errors::Result<()> {
        let time_correction_enabled = self.is_time_correction_enabled();
        let client_name = String::from(self.get_name());

        if time_correction_enabled {
            crate::set_time_skew(0.0);
        }

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
        trace!("wrote to stream; success={:?}", result);
        trace!("Wrote: {:x?}", &self.write_buffer[0..len]);

        let result = stream.read(&mut self.read_buffer).await;

        if let Ok(size) = result {
            trace!("Read: {}", size);
        } else {
            trace!("Error: {:?} ", result);
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

    async fn read_loop(
        mut reader: tokio::io::ReadHalf<tokio::net::TcpStream>,
        mut outbox: tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> errors::Result<()> {
        // TODO: Need to set the size from a configuration
        let mut read_buffer = vec![0; 200000];
        let mut time = std::time::Instant::now();
        loop {
            trace!("read_loop");
            // TODO: Need to move the timing into a separate task
            if time.elapsed() > std::time::Duration::from_millis(1000) {
                warn!("Sending timing message");
                time = std::time::Instant::now();
                // TODO: Get the client name
                let timing = Message::timing("umm-1");
                if let Err(e) = outbox.send(timing) {
                    error!("Failed to send the timing message. {}", e);
                }
            }

            let result = reader.read(&mut read_buffer).await;

            let (msg_list, _) = if let Ok(bytes_read) = result {
                if bytes_read > 0 {
                    crate::message::decode_slice(&read_buffer)?
                } else {
                    continue;
                }
            } else {
                return Err(errors::Error::General("Failed to decode message."));
            };

            trace!("Received {} messages.", msg_list.len());

            for message in msg_list {
                trace!("Received Message of type: {}", message);
            }
        }
    }

    async fn write_loop(
        mut writer: tokio::io::WriteHalf<tokio::net::TcpStream>,
        mut outbox: tokio::sync::mpsc::UnboundedReceiver<Message>,
    ) {
        // TODO: Need to set the size from a configuration
        let mut write_buffer = vec![0; 200000];
        loop {
            if let Some(message) = outbox.recv().await {
                // TODO: Don't use unwrap
                let len = crate::message::encode_slice(&message, &mut write_buffer).unwrap();

                let result = writer.write(&mut write_buffer[0..len]).await;
                if let Err(e) = result {
                    error!("Failed to write a message: {}", e);
                    if let Err(ee) = writer.shutdown().await {
                        error!(
                            "Failed to shutdown client after failing to send.. Double fail: {}",
                            ee
                        );
                    }
                }
            }
        }
    }
}

impl Publish<f64> for AsyncClient {
    fn publish(&mut self, key: &str, value: f64) -> errors::Result<()> {
        let mut message = Message::notify_double(key, value, crate::time_warped());
        return self.send_message(message);
    }
}

impl Publish<&Vec<u8>> for AsyncClient {
    fn publish(&mut self, key: &str, value: &Vec<u8>) -> errors::Result<()> {
        let mut message = Message::notify_data(key, value, crate::time_warped());
        return self.send_message(message);
    }
}

impl Publish<String> for AsyncClient {
    fn publish(&mut self, key: &str, value: String) -> errors::Result<()> {
        let mut message = Message::notify_string(key, value, crate::time_warped());
        return self.send_message(message);
    }
}

impl Publish<&str> for AsyncClient {
    fn publish(&mut self, key: &str, value: &str) -> errors::Result<()> {
        let mut message = Message::notify_string(key, value, crate::time_warped());
        return self.send_message(message);
    }
}
