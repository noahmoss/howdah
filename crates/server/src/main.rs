use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use howdah_core::{QueryResult, run_query};
use nvim_rs::{Handler, Neovim, Value, compat::tokio::Compat, create::tokio as create};
use tokio::fs::File;
use tokio_postgres::{Client, NoTls};

#[derive(Clone, Debug)]
struct NeovimHandler {
    client: Arc<Client>,
}

#[async_trait]
impl Handler for NeovimHandler {
    type Writer = Compat<File>;

    async fn handle_request(
        &self,
        name: String,
        args: Vec<Value>,
        _neovim: Neovim<Compat<File>>,
    ) -> Result<Value, Value> {
        match name.as_ref() {
            "ping" => Ok(Value::from("pong")),
            "query" => {
                if args.len() == 1 {
                    if let Some(sql) = args[0].as_str() {
                        match run_query(&self.client, sql).await {
                            Ok(QueryResult { rows, cols }) => {
                                let col_value =
                                    Value::Array(cols.into_iter().map(Value::from).collect());

                                let row_values = Value::Array(
                                    rows.into_iter()
                                        .map(|row| {
                                            Value::Array(row.into_iter().map(Value::from).collect())
                                        })
                                        .collect(),
                                );
                                Ok(Value::Map(vec![
                                    (Value::from("cols"), col_value),
                                    (Value::from("rows"), row_values),
                                ]))
                            }
                            Err(err) => {
                                let mut err_text = format!("Execution error: {}\n", err);
                                let mut source = err.source();
                                while let Some(e) = source {
                                    err_text.push_str(&format!("Caused by: {}\n", e));
                                    source = e.source()
                                }
                                Err(Value::from(err_text))
                            }
                        }
                    } else {
                        Err(Value::from(format!(
                            "method {} expects string, received {}",
                            name, &args[0]
                        )))
                    }
                } else {
                    Err(Value::from(format!(
                        "method {} expects 1 arg, received {}",
                        name,
                        args.len()
                    )))
                }
            }
            _ => Err(Value::from(format!("unknown method: {}", name))),
        }
    }
}

// TODO: add error logging not to stderr
#[tokio::main]
async fn main() {
    let (client, connection) =
        tokio_postgres::connect("host=localhost user=noahmoss dbname=bird-flocks", NoTls)
            .await
            .expect("Failed to connect to PostgreSQL");

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let handler = NeovimHandler {
        client: Arc::new(client),
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
