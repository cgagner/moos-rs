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
use regex::Regex;
use std::collections::{HashMap, HashSet};
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
pub type WildcardMap = HashMap<String, HashSet<String>>;
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
    on_connect_callback: Option<fn()>,
    on_disconnect_callback: Option<fn()>,
    published_keys: HashSet<String>,
    subscribed_keys: HashSet<String>,
    wildcard_subscribed_keys: WildcardMap,
}

pub trait Publish<D> {
    fn publish(&mut self, key: &str, value: D) -> errors::Result<()>;
}

impl AsyncClient {
    /// Create a new asynchronous client with the specified name.
    ///
    /// Arguments:
    ///
    /// * `name`: Name of the client
    pub fn new<S>(name: S) -> Self
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
            on_connect_callback: None,
            on_disconnect_callback: None,
            published_keys: HashSet::new(),
            subscribed_keys: HashSet::new(),
            wildcard_subscribed_keys: HashMap::new(),
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

    /// Get the keys published by the client.
    /// Returns: Borrow of the [`HashSet<String>`] that have been publisehd
    ///          by the client.
    pub fn get_published_keys(&self) -> &HashSet<String> {
        return &self.published_keys;
    }

    /// Get the keys subscribed to by the client.
    /// Returns: Borrow of the [`HashSet<String>`] that has been subscribed
    ///          to by the client.
    pub fn get_subscribed_keys(&self) -> &HashSet<String> {
        return &self.subscribed_keys;
    }

    /// Get the map of wildcard subscriptions
    /// Returns: Borrow of the [`WildcardMap`] of wildcard subscriptions
    pub fn get_wildcard_subscribed_keys(&self) -> &WildcardMap {
        return &self.wildcard_subscribed_keys;
    }

    /// Check if the specified key is in the list of being subscribed to by
    /// the client.
    pub fn is_subscribed_to(&self, key: &str) -> bool {
        self.subscribed_keys.contains(key)
            || self.wildcard_subscribed_keys.contains_key(key)
            || self
                .wildcard_subscribed_keys
                .keys()
                .any(|pattern| Self::is_wildcard_match(pattern, key))
    }

    pub fn is_time_correction_enabled(&self) -> bool {
        /// TODO
        return true;
    }

    /// Set the callback to be called when the client is conected.
    ///
    /// ```
    /// use moos::async_client::AsyncClient;
    /// let mut client = AsyncClient::new("ClientName");
    /// client.set_on_connect(move || {
    ///     log::error!("Client Connected!");
    /// });
    /// ```
    pub fn set_on_connect(&mut self, on_connect: fn()) {
        self.on_connect_callback = Some(on_connect);
    }

    /// Set the callback to be called when the client is disconected.
    ///
    /// ```
    /// use moos::async_client::AsyncClient;
    /// let mut client = AsyncClient::new("ClientName");
    /// client.set_on_disconnect(move || {
    ///     log::error!("Client Disconnected!");
    /// });
    /// ```
    pub fn set_on_disconnect(&mut self, on_disconnect: fn()) {
        self.on_disconnect_callback = Some(on_disconnect);
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
                let _ = self.disconnect().await;
                return Err(errors::Error::General(
                    "AsyncClient somehow got an invalid stream while connecting.",
                ));
            }
        };

        if let Err(e) = self.handshake(&mut stream).await {
            let _ = self.disconnect().await;
            return Err(e);
        }

        self.last_connected_time = time_warped();

        if let Some(on_connect) = self.on_connect_callback {
            on_connect();
        }

        let (mut reader, mut writer) = tokio::io::split(stream);

        let (outbox, rx) = mpsc::unbounded_channel::<Message>();

        let reader_outbox = outbox.clone();
        let inbox = self.inbox.clone();
        tokio::spawn(async move {
            let _ = AsyncClient::read_loop(reader, reader_outbox, inbox).await;
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

        if let Some(on_disconnect) = self.on_disconnect_callback {
            on_disconnect();
        }

        Ok(())
    }

    /// Check if a key contains wildcards.
    pub(crate) fn is_wildcard(key: &str) -> bool {
        key.contains("*") || key.contains("?")
    }

    /// Check if a pattern matches a key.
    ///
    /// Arguments:
    ///
    /// `key_pattern`: pattern to check. The pattern should
    ///                match the MOOS wildcard characters.
    /// `test_key`: key to test
    ///
    /// Returns: true if the pattern matches the key.
    pub(crate) fn is_wildcard_match(key_pattern: &str, test_key: &str) -> bool {
        if let Ok(re) = Regex::new(
            format!("^{}$", key_pattern)
                .replace("*", ".*")
                .replace("?", ".?")
                .as_str(),
        ) {
            re.is_match(test_key)
        } else {
            false
        }
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
        if Self::is_wildcard(key) {
            return self.subscribe_from(key, "*", interval);
        }

        let message = Message::register(self.get_name(), key, interval);

        // TODO: May need to store the filter - when we add filters

        let result = self.send_message(message);
        if let Ok(()) = result {
            self.subscribed_keys.insert(key.to_string());
        }
        return result;
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

        let message = Message::wildcard_register(self.get_name(), string_value.as_str());

        let result = self.send_message(message);

        if let Ok(()) = result {
            // TODO: May need to store the filter
            if let Some(apps) = self.wildcard_subscribed_keys.get_mut(key) {
                apps.insert(app_pattern.to_string());
            } else {
                self.wildcard_subscribed_keys.insert(
                    key.to_string(),
                    vec![app_pattern.to_string()].into_iter().collect(),
                );
            }
        }

        return result;
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

        if key.is_empty() {
            return Err(errors::Error::General(
                "Cannot unsubscribe to an empty key string.",
            ));
        }

        // If the key contains a wildcard character, use the unsubscribe_from
        // method to handle sending a wildcard register
        if Self::is_wildcard(key) {
            return self.unsubscribe_from(key, "*");
        }

        // If we haven't subscibed to the key, we don't need to send
        // a message to unsubscibe.
        if !self.subscribed_keys.contains(key) {
            log::info!(
                "Cannot unsubscribe to {} because we never subscribed to it.",
                key
            );
            return Ok(());
        }

        let message = Message::unregister(self.get_name(), key, 0.0);

        let result = self.send_message(message);

        if let Ok(()) = result {
            self.subscribed_keys.remove(key);
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

        // If the key contains a wildcard character, check if we subscribed
        // to the key. If not, we are done.
        if !Self::is_wildcard(key)
            && !self.subscribed_keys.contains(key)
            && !self.wildcard_subscribed_keys.contains_key(key)
        {
            log::info!(
                "Cannot unsubscribe to {} from {} because we never subscribed to it.",
                key,
                app_pattern
            );
            return Ok(());
        }

        let apps_option = self.wildcard_subscribed_keys.get_mut(key);
        if let Some(apps) = apps_option {
            if app_pattern.ne("*") && !apps.contains(app_pattern) {
                log::info!(
                    "Cannot unsubscribe to {} from {} because we never subscribed to it. Could not find app_pattern",
                    key,
                    app_pattern
                );
                return Ok(());
            }
        } else {
            log::info!(
                "Cannot unsubscribe to {} from {} because we never subscribed to it. Could not find key.",
                key,
                app_pattern
            );
            return Ok(());
        }

        let string_value = format!(
            "AppPattern={},VarPattern={},Interval={}",
            app_pattern, key, 0.0
        );

        // TODO: If the app_pattern is a wildcard, we should unsubscribe to all.
        // That may mean we need to send multiple messages. E.G. for app_a and for app_b.

        let message = Message::wildcard_unregister(self.get_name(), string_value.as_str());

        let result = self.send_message(message);

        if let Ok(()) = result {
            if let Some(apps) = self.wildcard_subscribed_keys.get_mut(key) {
                apps.remove(app_pattern);
                if apps.is_empty() {
                    self.wildcard_subscribed_keys.remove(key);
                }
            }
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
                if let Err(e) = result {
                    // TODO: Need to figure out how to call shutdown
                    // match e.kind() {
                    //
                    // }
                }
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
                // TODO: Check to see if we've sent at timeine message recently
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
                        break;
                        // TODO: Need to figure out how to how to call disconnect
                    }
                }
            }
        }
    }
}

impl Publish<f64> for AsyncClient {
    fn publish(&mut self, key: &str, value: f64) -> errors::Result<()> {
        let message = Message::notify_double(key, value, crate::time_warped());
        let result = self.send_message(message);
        if let Ok(()) = result {
            self.published_keys.insert(key.to_string());
        }
        return result;
    }
}

impl Publish<&Vec<u8>> for AsyncClient {
    fn publish(&mut self, key: &str, value: &Vec<u8>) -> errors::Result<()> {
        let message = Message::notify_data(key, value, crate::time_warped());
        let result = self.send_message(message);
        if let Ok(()) = result {
            self.published_keys.insert(key.to_string());
        }
        return result;
    }
}

impl Publish<String> for AsyncClient {
    fn publish(&mut self, key: &str, value: String) -> errors::Result<()> {
        let message = Message::notify_string(key, value, crate::time_warped());
        let result = self.send_message(message);
        if let Ok(()) = result {
            self.published_keys.insert(key.to_string());
        }
        return result;
    }
}

impl Publish<&str> for AsyncClient {
    fn publish(&mut self, key: &str, value: &str) -> errors::Result<()> {
        let message = Message::notify_string(key, value, crate::time_warped());
        let result = self.send_message(message);
        if let Ok(()) = result {
            self.published_keys.insert(key.to_string());
        }
        return result;
    }
}

#[cfg(test)]
mod tests {
    use crate::async_client::AsyncClient;
    #[test]
    fn test_wildcard_match() {
        assert!(AsyncClient::is_wildcard_match("NAV_*", "NAV_X"));
        assert!(AsyncClient::is_wildcard_match("NAV_*", "NAV_"));
        assert!(AsyncClient::is_wildcard_match("NAV_*", "NAV_*"));
        assert!(AsyncClient::is_wildcard_match("NAV_*", "NAV_?"));
        assert!(AsyncClient::is_wildcard_match("NAV_*", "NAV_DEPTH"));
        assert!(!AsyncClient::is_wildcard_match("NAV_*", "ANAV_X"));
        assert!(!AsyncClient::is_wildcard_match("NAV_*", "ASDF"));
        assert!(AsyncClient::is_wildcard_match("NAV_*_TEST", "NAV__TEST"));
        assert!(AsyncClient::is_wildcard_match(
            "NAV_*_TEST",
            "NAV_GOOD_TEST"
        ));
        assert!(!AsyncClient::is_wildcard_match("NAV_*_TEST", "NAV_X"));
        assert!(AsyncClient::is_wildcard_match("*V_X", "NAV_X"));

        assert!(AsyncClient::is_wildcard_match("NAV_?", "NAV_X"));
        assert!(AsyncClient::is_wildcard_match("NAV_?", "NAV_"));
        assert!(AsyncClient::is_wildcard_match("NAV_?", "NAV_*"));
        assert!(AsyncClient::is_wildcard_match("NAV_?", "NAV_?"));
        assert!(!AsyncClient::is_wildcard_match("NAV_?", "NAV_DEPTH"));
        assert!(!AsyncClient::is_wildcard_match("NAV_?", "ASDF"));
        assert!(AsyncClient::is_wildcard_match("N?V_X", "NAV_X"));
        assert!(AsyncClient::is_wildcard_match("N?V_X", "NOV_X"));
        assert!(!AsyncClient::is_wildcard_match("N?V_X", "NOOV_X"));
        assert!(!AsyncClient::is_wildcard_match("TEST", "ASDF"));
        assert!(!AsyncClient::is_wildcard_match("NOOV_X", "NAV_X"));
    }
}
