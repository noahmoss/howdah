use std::sync::{Arc, Mutex};

use handlers::{NeovimHandler, error_chain};
use nvim_rs::create::tokio as create;

mod handlers;

// TODO: add error logging not to stderr
#[tokio::main]
async fn main() {
    let handler = NeovimHandler {
        client: Arc::new(Mutex::new(None)),
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
                eprintln!("Error: {}", error_chain(&err))
            }

            // Otherwise, assume this is a part of normal shutdown
        }

        // Clean shutdown
        Ok(Ok(())) => {}
    }
}
