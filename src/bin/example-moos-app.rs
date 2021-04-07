extern crate moos;

use std::{str, str::FromStr, thread::sleep};

use crate::moos::async_client::AsyncClient;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::join;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // Open a TCP stream to the socket address.
    //
    // Note that this is the Tokio TcpStream, which is fully async.

    let mut client = AsyncClient::new("umm-1").await;
    if let Ok(()) = client.handshake().await {
        println!("Connected! Community: {}", client.get_community());
    }

    let task1 = tokio::spawn(async move {
        loop {
            println!("Task running1");
            if let Ok(()) = client.handshake().await {
                println!("Connected! Community: {}", client.get_community());
            }
            client.disconnect().await;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    let task2 = tokio::spawn(async move {
        loop {
            println!("Task running2");
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    });

    join!(task1, task2);

    Ok(())
}
