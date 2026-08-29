use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use handlers::NeovimHandler;
use nvim_rs::create::tokio as create;
use tokio_postgres::NoTls;

mod handlers;

// TODO: add error logging not to stderr
#[tokio::main]
async fn main() {
    let (client, connection) =
        tokio_postgres::connect("host=localhost user=noahmoss dbname=howdah_dev", NoTls)
            .await
            .expect("Failed to connect to PostgreSQL");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let handler = NeovimHandler {
        client: Arc::new(Mutex::new(Some(Arc::new(client)))),
    };
    let (nvim, io_handler) = create::new_parent(handler).await.unwrap();

    match io_handler.await {
        Err(joinerr) => eprintln!("Error joining IO loop: '{}'", joinerr),
        Ok(Err(err)) => {
            // Nvim still present; try to alert the user of the error
            if !err.is_reader_error() {
                nvim.err_writeln(&format!("Error: {}", err))
                    .await
                    .unwrap_or_else(|e| eprintln!("Error: {}", e))
            }

            // Not a clean channel close; walk & log the error sources
            if !err.is_channel_closed() {
                eprintln!("Error: {}", err);

                let mut source = err.source();
                while let Some(e) = source {
                    eprintln!("Caused by: {}", e);
                    source = e.source();
                }
            }

            // Otherwise, assume this is a part of normal shutdown
        }

        // Clean shutdown
        Ok(Ok(())) => {}
    }
}
