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
    use std::{cmp::Ordering, str::from_utf8};

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

        let res = child.join();

        println!("TimeWarp: {}", get_time_warp());
        assert!((get_time_warp() - 2.0).abs() < 1e-9);
    }
}
