pub mod errors;
pub mod message;

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
}
