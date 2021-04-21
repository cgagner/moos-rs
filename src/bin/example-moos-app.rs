extern crate moos;

use std::{env, str, str::FromStr, thread::sleep};

use crate::moos::async_client::AsyncClient;
use moos::async_client::Publish;
use simple_logger::SimpleLogger;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::{join, task};

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new().init().unwrap();
    // Open a TCP stream to the socket address.
    //
    // Note that this is the Tokio TcpStream, which is fully async.

    let args: Vec<String> = env::args().collect();

    let mut client_name: String = "umm-1".into();
    let mut sub_vars = Vec::<String>::new();
    for arg in args {
        //
        if arg.starts_with("-") || arg.starts_with("--") {
            //
            let a = arg.trim_start_matches("-");
            let (name, opt) = if let Some((name, opt)) = a.split_once("=") {
                (name, opt)
            } else {
                (a, "")
            };

            match name {
                "moos_name" => client_name = opt.into(),
                "s" => sub_vars.push(opt.into()),
                _ => log::error!("Unknown argument: {}", name),
            }
        }
    }

    log::trace!("Client name: {}", client_name);

    let mut client = AsyncClient::new(client_name).await;
    if let Ok(()) = client.connect().await {
        println!("Connected! Community: {}", client.get_community());
    }

    for s in sub_vars {
        client.subscribe(&s, 0.0);
    }

    let task1 = tokio::spawn(async move {
        loop {
            println!("Task running1");

            client.publish("TEST_12", "TRUE");

            println!("Finished publishing RETURN");

            // TODO: Need to update the client to periodically sent a heartbeat message.

            // if let Err(e) = client.disconnect().await {
            //     eprintln!("Failed to disconnect! {:?}", e);
            //     return;
            // }
            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
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
