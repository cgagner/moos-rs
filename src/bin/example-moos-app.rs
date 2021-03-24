extern crate moos;

use std::{str, str::FromStr};

use crate::moos::message::{Message, ValueType};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // Open a TCP stream to the socket address.
    //
    // Note that this is the Tokio TcpStream, which is fully async.
    let mut stream = TcpStream::connect("127.0.0.1:9000").await?;
    println!("created stream");

    let result = stream
        .write(moos::message::PROTOCOL_CONNECT_MESSAGE.as_bytes())
        .await;
    println!("wrote to stream; success={:?}", result.is_ok());

    let mut write_buf = [0; 20000];

    // TODO: Serialize a packet

    let msg = moos::message::Message::connect();

    let len = moos::message::encode_slice(msg, &mut write_buf).unwrap();

    let result = stream.write(&mut write_buf[0..len]).await;
    println!("wrote to stream; success={:?}", result);
    println!("Wrote: {:x?}", &write_buf[0..len]);

    let mut read_buf = [0; 20000];

    let result = stream.read(&mut read_buf).await;

    if let Ok(size) = result {
        println!("Read: {}", size);
    } else {
        println!("Error: {:?} ", result);
    }

    Ok(())
}
