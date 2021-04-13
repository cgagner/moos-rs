extern crate moos;

use std::{str, str::FromStr, thread::sleep};

use crate::moos::async_client::AsyncClient;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{join, task};

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // Open a TCP stream to the socket address.
    //
    // Note that this is the Tokio TcpStream, which is fully async.

    let mut client = AsyncClient::new("umm-1").await;
    if let Ok(()) = client.connect().await {
        println!("Connected! Community: {}", client.get_community());
    }

    let task1 = tokio::spawn(async move {
        loop {
            println!("Task running1");
            let result = client.connect().await;
            match result {
                Ok(()) => println!("Connected! Community: {}", client.get_community()),
                Err(e) => eprintln!("Failed to connect! {:?}", e),
            }

            client.subscribe("DB_CLIENTS", 0.0).await;
            client.read_loop().await;

            // TODO: Need to update the client to periodically sent a heartbeat message.

            if let Err(e) = client.disconnect().await {
                eprintln!("Failed to disconnect! {:?}", e);
                return;
            }
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
