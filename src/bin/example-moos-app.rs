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
    let mut wildcard_sub_vars = Vec::<String>::new();
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
                "w" => wildcard_sub_vars.push(opt.into()),
                _ => log::error!("Unknown argument: {}", name),
            }
        }
    }

    log::trace!("Client name: {}", client_name);

    let mut client = AsyncClient::new(client_name);

    client.set_on_connect(|| {
        log::error!("Client Connected!!");
    });

    client.set_on_disconnect(|| {
        log::error!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\nClient Disconnected!!");
        // TODO: This is for testing.. Don't really do this
        std::process::exit(-1);
    });

    if let Ok(()) = client.connect().await {
        println!("Connected! Community: {}", client.get_community());
    }

    let mut inbox = client.start_consuming();

    for s in sub_vars {
        // TODO: We should parse the interval from the command line arguments
        client.subscribe(&s, 0.0);
    }

    for w in &wildcard_sub_vars {
        // TODO: We should parse the interval and app_pattern from the command line arguments
        log::error!("Wildcard subcription: {}", w);
        client.subscribe_from(&w, "*", 0.0);
    }

    let mut counter = 0_i32;

    let task1 = tokio::spawn(async move {
        loop {
            log::info!("Task running1");

            client.publish("TEST_12", "TRUE");

            for message in inbox.try_iter() {
                log::error!("Received Message: {}", message);
            }

            log::info!("Finished publishing RETURN");

            if counter == 4 {
                log::error!("Testing unsubscribe");
                client.unsubscribe("TEST_12");
            }

            if counter == 10 {
                log::error!("Testing wildcard unsubscribe");
                for w in &wildcard_sub_vars {
                    client.unsubscribe_from(w, "*");
                }
            }
            // if counter == 5 {
            //     log::error!(
            //         "Testing stopping the comsumer to see if the client handles it gracefully."
            //     );
            //     client.stop_consuming();
            // }
            counter += 1;
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
