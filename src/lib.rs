pub mod async_client;
pub mod errors;
pub mod message;

use std::sync::{Arc, Once, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum allowable time difference between client clock and MOOSDB clock. 5 Seconds.
const SKEW_TOLERANCE: f64 = 5.0;

pub fn time_local(apply_time_warp: bool) -> f64 {
    let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_secs_f64(),
        Err(_) => 0.0,
    };
    if apply_time_warp {
        time * get_time_warp()
    } else {
        time
    }
}

pub fn time_unwarped() -> f64 {
    time_local(false) + get_time_skew()
}

pub fn time_warped() -> f64 {
    time_local(true) + get_time_skew()
}

pub fn get_time_warp() -> f64 {
    *get_safe_time_warp().read().unwrap()
}

pub fn set_time_warp(time_warp: f64) {
    *get_safe_time_warp().write().unwrap() = time_warp
}

/// Get the skew
pub fn get_time_skew() -> f64 {
    *get_safe_time_skew().read().unwrap()
}

/// Set the skew between the local client and the MOOS DB.
pub fn set_time_skew(time_skew: f64) {
    *get_safe_time_skew().write().unwrap() = time_skew
}

fn get_safe_time_warp() -> Arc<RwLock<f64>> {
    static mut SINGLETON: *const Arc<RwLock<f64>> = 0 as _;
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            // Make it
            let singleton = Arc::new(RwLock::new(1.0));

            // Put it in the heap so it can outlive this call
            SINGLETON = std::mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*SINGLETON).clone()
    }
}

fn get_safe_time_skew() -> Arc<RwLock<f64>> {
    static mut SINGLETON: *const Arc<RwLock<f64>> = 0 as _;
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            // Make it
            let singleton = Arc::new(RwLock::new(0.0));

            // Put it in the heap so it can outlive this call
            SINGLETON = std::mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*SINGLETON).clone()
    }
}

#[cfg(test)]
mod tests {
    use simple_logger::SimpleLogger;

    use crate::{
        async_client::{AsyncClient, Publish},
        message::{Message, ValueType},
    };
    use std::sync::atomic::AtomicU16;
    use std::{cmp::Ordering, time::Duration};
    use std::{
        process::{Child, Command},
        sync::mpsc::Receiver,
    };

    #[test]
    fn it_works() {
        let s = String::from("asdf💖");

        println!("String: {}, ", s);
        println!("s.len() {}", s.len());
        println!("s.chars().count: {}", s.chars().count());
        println!("s.bytes(): {:?}", s.as_bytes());

        let buf = vec![97, 115, 100, 102, 240, 159, 146, 150];
        let ss = String::from_utf8(buf).unwrap();

        assert_eq!(s.cmp(&ss), Ordering::Equal);

        let buf2: [u8; 8] = [97, 115, 100, 102, 240, 159, 146, 150];

        let ss = std::str::from_utf8(&buf2).unwrap_or("");

        assert_eq!(ss.cmp(&s), Ordering::Equal)
    }

    #[test]
    fn test_set_time_warp() {
        use crate::{get_time_warp, set_time_warp};
        use std::thread;

        set_time_warp(5.0);
        assert!((get_time_warp() - 5.0).abs() < 1e-9);
        // Setting the time warp from a seond thread should be still result
        // in the global setting update
        let child = thread::spawn(move || set_time_warp(2.0));

        child
            .join()
            .expect("Failed to child child thread in test_set_time_warp");

        println!("TimeWarp: {}", get_time_warp());
        assert!((get_time_warp() - 2.0).abs() < 1e-9);
    }

    /// AtomicU16 to use to create a new port for each test since the
    /// tests run in parallel.
    static PORT: AtomicU16 = AtomicU16::new(9700);

    /// Get a new port number for connecting to the MOOSDB
    fn get_new_port() -> u16 {
        PORT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Helper Structure that will start the MOOSDB on creation and will kill
    /// the MOOSDB when dropped. This works even if there is a panic!
    struct MoosDBController {
        child: Child,
    }

    impl MoosDBController {
        pub fn new(port: u16) -> Self {
            let child = Command::new("MOOSDB")
                .arg(format!("--moos_port={}", port))
                .spawn()
                .expect(
                    format!(
                        "ERROR! Failed to start the MOOSDB on {}. Is it in your path?",
                        port
                    )
                    .as_str(),
                );

            MoosDBController { child }
        }

        fn is_running(&mut self) -> bool {
            let status = self.child.try_wait().unwrap();
            status.is_none()
        }
    }

    impl Drop for MoosDBController {
        fn drop(&mut self) {
            self.child.kill().expect("Failed to kill MOOSDB");
        }
    }

    fn setup_moosdb(port: u16) -> Option<MoosDBController> {
        // **NOTE:** Don't use catch_unwind in a real application. We're only
        // using it here so the tests won't fail if the MOOSDB can't be found.
        // Hopefully, this will go away when we figure out how to add the MOOSDB
        // to the GitHub Actions.
        if let Ok(child) = std::panic::catch_unwind(|| MoosDBController::new(port)) {
            return Some(child);
        } else {
            return None;
        };
    }

    async fn setup_client(port: u16, name: &str) -> (AsyncClient, Receiver<Message>) {
        let mut client = AsyncClient::new(name);

        let receiver = client.start_consuming();
        // TODO: Need to separate out the connect method from the connect loop. Setting
        // this to an invalid port should return after some timeout.
        if let Err(e) = client.connect_to("localhost", port).await {
            eprintln!("Received an error during setup_client: {:?}", e);
            assert!(false);
        }
        (client, receiver)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn int_test_subscibe() {
        let _ = SimpleLogger::new().init();
        let port = get_new_port();
        let mut child = if let Some(child) = setup_moosdb(port) {
            child
        } else {
            return;
        };
        assert!(child.is_running());

        let client_name = "int_test_subscribe";
        let key1 = "NAV_X";
        let key2 = "NAV_Y";

        let (mut client, receiver) = setup_client(port, client_name).await;

        client
            .subscribe(key2, 0.0)
            .expect(format!("Failed to subscibe to {}", key2).as_str());

        assert!(client.get_subscribed_keys().contains(key2));
        assert!(client.is_subscribed_to(key2));
        // This should fail since we haven't subscribed to it yet.
        assert!(!client.get_subscribed_keys().contains(key1));
        assert!(!client.is_subscribed_to(key1));

        client
            .subscribe(key1, 0.0)
            .expect(format!("Failed to subscibe to {}", key1).as_str());

        assert!(client.get_subscribed_keys().contains(key2));
        assert!(client.is_subscribed_to(key2));
        assert!(client.get_subscribed_keys().contains(key1));
        assert!(client.is_subscribed_to(key1));

        // Unsubscribe
        client
            .unsubscribe(key2)
            .expect(format!("Failed to unsubscibe to {}", key2).as_str());

        assert!(!client.get_subscribed_keys().contains(key2));
        assert!(!client.is_subscribed_to(key2));
        assert!(client.get_subscribed_keys().contains(key1));
        assert!(client.is_subscribed_to(key1));

        client
            .unsubscribe(key1)
            .expect(format!("Failed to unsubscibe to {}", key1).as_str());

        assert!(!client.get_subscribed_keys().contains(key2));
        assert!(!client.is_subscribed_to(key2));
        assert!(!client.get_subscribed_keys().contains(key1));
        assert!(!client.is_subscribed_to(key1));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn int_test_subscibe_from() {
        let _ = SimpleLogger::new().init();
        let port = get_new_port();
        let mut child = if let Some(child) = setup_moosdb(port) {
            child
        } else {
            return;
        };

        assert!(child.is_running());

        let client_name = "int_test_subscribe_from";
        let key1 = "NAV_X";
        let key2 = "NAV_*";

        let (mut client, receiver) = setup_client(port, client_name).await;

        assert!(!client.is_subscribed_to(key1));
        assert!(!client.is_subscribed_to(key2));

        client
            .subscribe_from(key1, client_name, 0.0)
            .expect(format!("Failed to {} subscribe_from {}", key1, client_name).as_str());

        assert!(client.is_subscribed_to(key1));
        assert!(!client.is_subscribed_to(key2));

        client
            .subscribe_from(key2, client_name, 0.0)
            .expect(format!("Failed to {} subscribe_from {}", key2, client_name).as_str());

        assert!(client.is_subscribed_to(key1));
        assert!(client.is_subscribed_to(key2));

        client
            .unsubscribe_from(key1, client_name)
            .expect(format!("Failed to {} unsubscribe_from {}", key1, client_name).as_str());

        // Key2 is a wildcard. We should still be subscribed to key1
        assert!(!client.get_wildcard_subscribed_keys().contains_key(key1));
        assert!(client.is_subscribed_to(key1));
        assert!(client.get_wildcard_subscribed_keys().contains_key(key2));
        assert!(client.is_subscribed_to(key2));

        client
            .unsubscribe_from(key2, client_name)
            .expect(format!("Failed to {} unsubscribe_from {}", key2, client_name).as_str());

        // Key2 is a wildcard. We should still be subscribed to key1
        assert!(!client.get_wildcard_subscribed_keys().contains_key(key1));
        assert!(!client.is_subscribed_to(key1));
        assert!(!client.get_wildcard_subscribed_keys().contains_key(key2));
        assert!(!client.is_subscribed_to(key2));

        let publisher_name = "test_publisher";

        client
            .subscribe_from(key1, publisher_name, 0.0)
            .expect(format!("Failed to {} subscribe_from {}", key1, publisher_name).as_str());

        assert!(client.get_wildcard_subscribed_keys().contains_key(key1));
        assert!(client.is_subscribed_to(key1));

        let (mut client2, _) = setup_client(port, publisher_name).await;
        let test_value = 1234.4321;
        client2
            .publish(key1, test_value)
            .expect(format!("Failed to publish {} from {}", key1, publisher_name).as_str());

        if let Ok(message) = receiver.recv_timeout(Duration::from_secs(1)) {
            assert!(message.is_notify());
            assert!(message.key() == key1);
            assert!(message.source() == publisher_name);
            match message.value() {
                ValueType::Double(d) => assert!((d - test_value).abs() < 0.001),
                _ => assert!(false),
            };
        } else {
            assert!(false);
        }

        // TODO: Add test publish from a different client and verify we don't get it.

        client2
            .disconnect()
            .await
            .expect(format!("Failed to disconnect {}", client2.get_name()).as_str());
        client
            .disconnect()
            .await
            .expect(format!("Failed to disconnect {}", client.get_name()).as_str());
    }
}
