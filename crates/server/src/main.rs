use std::error::Error;

use async_trait::async_trait;
use nvim_rs::{Handler, Neovim, Value, compat::tokio::Compat, create::tokio as create};
use tokio::fs::File;

#[derive(Clone, Debug)]
struct NeovimHandler {}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Compat<File>;

    async fn handle_request(
        &self,
        name: String,
        _args: Vec<Value>,
        _neovim: Neovim<Compat<File>>,
    ) -> Result<Value, Value> {
        match name.as_ref() {
            "ping" => Ok(Value::from("pong")),
            _ => Err(Value::from(format!("unknown method: {}", name))),
        }
    }
}

// TODO: add error logging not to stderr
#[tokio::main]
async fn main() {
    let handler = NeovimHandler {};
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

            // Otherwise, assume this is aprt of normal shutdown
        }

        // Clean shutdown
        Ok(Ok(())) => {}
    }
}
