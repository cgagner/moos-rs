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
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{
    net::{Shutdown, SocketAddr},
    ops::DerefMut,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::{mpsc, mpsc::UnboundedReceiver, mpsc::UnboundedSender},
    time::sleep,
};

type Inbox = Arc<Mutex<Option<std::sync::mpsc::Sender<Message>>>>;

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
    outbox: Option<UnboundedSender<Message>>,
    inbox: Inbox,
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
            inbox: Arc::new(Mutex::new(None)),
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
    ///
    /// Arguments:
    ///
    /// * `host`: hostname or IP address of the MOOS database
    /// * `port`: port of the MOOS database
    ///
    /// Returns: [`Err`] if the connection fails.
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
        let client_name: String = self.get_community().into();
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

        let (outbox, rx) = mpsc::unbounded_channel::<Message>();

        let reader_outbox = outbox.clone();
        let inbox = self.inbox.clone();
        tokio::spawn(async move {
            AsyncClient::read_loop(reader, reader_outbox, inbox).await;
        });

        tokio::spawn(async move { AsyncClient::write_loop(writer, rx, client_name).await });

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

    /// Subscribe to messages with the specified key at the specified interval.
    ///
    /// Arguments:
    ///
    /// * `key`: Key of the messages the client is subscribing to. **NOTE:**
    ///          If the key contains wildcard characters (`*` or `?`), the
    ///          client will perform a wildcard subscription.
    /// * `interval`: Interval at which the client should receive messages.
    ///               0.0 to receive messages as soon as the value is chagned.
    ///
    /// Returns: [`errors::Error::General`] if the key is empty or the client is
    /// not connected.
    pub fn subscribe(&mut self, key: &str, interval: f64) -> errors::Result<()> {
        if !self.is_connected() {
            return Err(errors::Error::General(
                "AsyncClient::subscribe: cannot subscribe because the client is not connected.",
            ));
        }

        if key.is_empty() {
            return Err(errors::Error::General(
                "Cannot subscribe to an empty key string.",
            ));
        }

        // If the key contains a wildcard character, use the subscribe_from
        // method to handle sending a wildcard register
        if key.contains("*") || key.contains("?") {
            return self.subscribe_from(key, "*", interval);
        }

        let mut message = Message::register(self.get_name(), key, interval);

        // TODO: Need to store the variable being registered. May also
        // need to store the filter

        return self.send_message(message);
    }

    /// Subscribe to messages from a specific application with the specified
    /// key at the specified interval.
    ///
    /// Arguments:
    ///
    /// * `key`: Key of the messages the client is subscribing to. **NOTE:**
    ///          If the key contains wildcard characters (`*` or `?`), the
    ///          client will perform a wildcard subscription.
    /// * `app_pattern`: Application or applications to receive messages from.
    ///                  This can contain wildcard characters (`*` or `?`) to
    ///                  subscribe to multiple clients. This filtering happens
    ///                  at the MOOSDB side.
    /// * `interval`: Interval at which the client should receive messages.
    ///               0.0 to receive messages as soon as the value is chagned.
    ///
    /// Returns: [`errors::Error::General`] if the `key` or `app_pattern` are
    /// empty or the client is not connected.
    pub fn subscribe_from(
        &mut self,
        key: &str,
        app_pattern: &str,
        interval: f64,
    ) -> errors::Result<()> {
        if !self.is_connected() {
            return Err(errors::Error::General(
                "AsyncClient::subscribe: cannot subscribe because the client is not connected.",
            ));
        }

        if key.is_empty() {
            return Err(errors::Error::General(
                "Cannot subscribe to an empty key string.",
            ));
        }

        if app_pattern.is_empty() {
            return Err(errors::Error::General(
                "Cannot subscribe to an empty app_pattern string.",
            ));
        }

        let string_value = format!(
            "AppPattern={},VarPattern={},Interval={}",
            app_pattern, key, interval
        );

        let mut message = Message::wildcard_register(self.get_name(), string_value.as_str());

        // TODO: Need to store the variable being registered. May also
        // need to store the filter

        return self.send_message(message);
    }

    /// Unsubscribe to messages with the specified `key`.
    ///
    /// ***NOTE***: It is still possible to receive a few messages after the
    ///             client successfully unsubscribes to messages with a given
    ///             `key`. This will happen if the MOOSDB has already queued
    ///             messages to be delivered to the client.
    ///
    /// Arguments:
    ///
    /// * `key`: Key of the messages the client is unsubscribing to. **NOTE:**
    ///          If the key contains wildcard characters (`*` or `?`), the
    ///          client will perform a wildcard unsubscription.
    ///
    /// Returns: [`errors::Error::General`] if the `key` is empty
    /// or the client is not connected.
    pub fn unsubscribe(&mut self, key: &str) -> errors::Result<()> {
        if !self.is_connected() {
            return Err(errors::Error::General(
                "AsyncClient::subscribe: cannot subscribe because the client is not connected.",
            ));
        }

        // TODO: Check if the key is in the list of registered keys

        if key.is_empty() {
            return Err(errors::Error::General(
                "Cannot unsubscribe to an empty key string.",
            ));
        }

        let mut message = Message::unregister(self.get_name(), key, 0.0);

        let result = self.send_message(message);

        if let Ok(()) = result {
            // TODO: Need to remove the variable being registered.
        }

        return result;
    }

    /// Unsubscribe to messages from a specific application with the
    /// specified `key`.
    ///
    /// ***NOTE***: It is still possible to receive a few messages after the
    ///             client successfully unsubscribes to messages with a given
    ///             `key`. This will happen if the MOOSDB has already queued
    ///             messages to be delivered to the client.
    ///
    /// Arguments:
    ///
    /// * `key`: Key of the messages the client is unsubscribing to. **NOTE:**
    ///          If the key contains wildcard characters (`*` or `?`), the
    ///          client will perform a wildcard unsubscription.
    /// * `app_pattern`: Application or applications to receive messages from.
    ///                  This can contain wildcard characters (`*` or `?`) to
    ///                  subscribe to multiple clients. This filtering happens
    ///                  at the MOOSDB side.
    ///
    /// Returns: [`errors::Error::General`] if the `key` or `app_pattern` are
    /// empty or the client is not connected.
    pub fn unsubscribe_from(&mut self, key: &str, app_pattern: &str) -> errors::Result<()> {
        if !self.is_connected() {
            return Err(errors::Error::General(
                "AsyncClient::subscribe: cannot subscribe because the client is not connected.",
            ));
        }

        // TODO: Check to see if the set of registered keys is empty

        if key.is_empty() {
            return Err(errors::Error::General(
                "Cannot unsubscribe to an empty key string.",
            ));
        }

        if app_pattern.is_empty() {
            return Err(errors::Error::General(
                "Cannot unsubscribe to an empty app_pattern string.",
            ));
        }

        let string_value = format!(
            "AppPattern={},VarPattern={},Interval={}",
            app_pattern, key, 0.0
        );

        let mut message = Message::wildcard_unregister(self.get_name(), string_value.as_str());

        let result = self.send_message(message);

        if let Ok(()) = result {
            // TODO: Need to remove the variable being registered.
        }

        return result;
    }

    pub fn start_consuming(&mut self) -> std::sync::mpsc::Receiver<Message> {
        let (tx, rx) = std::sync::mpsc::channel();
        if let Ok(inbox) = &mut self.inbox.lock() {
            inbox.replace(tx);
        }
        rx
    }

    pub fn stop_consuming(&mut self) {
        if let Ok(inbox) = &mut self.inbox.lock() {
            inbox.take();
        }
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
        mut outbox: UnboundedSender<Message>,
        inbox: Inbox,
    ) -> errors::Result<()> {
        // TODO: Need to set the size from a configuration
        let mut read_buffer = vec![0; 200000];
        loop {
            trace!("read_loop");

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

            // TODO: We probably should only lock the inbox if we receive notify messages

            if let Ok(i) = &mut inbox.lock() {
                if let Some(tx) = i.deref_mut() {
                    for message in msg_list {
                        if message.is_notify() {
                            if let Err(e) = tx.send(message) {
                                error!("Failed to put message into the inbox: {}", e);
                                i.take();
                                break;
                            }
                        }
                    }
                } else {
                    // TODO: Should we continue to print a warning?
                    warn!("AsyncClient is receiving messages, but no one is consuming them.");
                }
            }
        }
    }

    async fn write_loop(
        mut writer: tokio::io::WriteHalf<tokio::net::TcpStream>,
        mut outbox: UnboundedReceiver<Message>,
        client_name: String,
    ) {
        // TODO: Need to set the size from a configuration
        let mut write_buffer = vec![0; 200000];
        loop {
            let message = if let Ok(msg) =
                tokio::time::timeout(Duration::from_millis(1000), outbox.recv()).await
            {
                msg
            } else {
                // We haven't sent a message in a second. Send a heartbeat
                trace!("AsyncClient hasn't sent a message in over a second. Sending a heartbeat.");
                Some(Message::timing(client_name.as_str()))
            };
            if let Some(message) = message {
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
