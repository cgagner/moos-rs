extern crate moos;

use std::{str, str::FromStr};

use crate::moos::async_client::AsyncClient;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // Open a TCP stream to the socket address.
    //
    // Note that this is the Tokio TcpStream, which is fully async.

    let mut client = AsyncClient::new().await;
    if let Ok(()) = client.handshake().await {
        println!("Connected! Community: {}", client.get_community());
        return Ok(());
    }

    Ok(())
}
