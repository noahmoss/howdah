use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use howdah_core::{QueryResult, run_query};
use nvim_rs::{Handler, Neovim, Value, compat::tokio::Compat};
use tokio::fs::File;
use tokio_postgres::Client;

#[derive(Clone, Debug)]
pub struct NeovimHandler {
    pub client: Arc<Mutex<Option<Arc<Client>>>>,
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
            "query" => self.handle_query(args).await,
            _ => Err(Value::from(format!("unknown method: {}", name))),
        }
    }
}

impl NeovimHandler {
    async fn handle_query(&self, args: Vec<Value>) -> Result<Value, Value> {
        if args.len() == 1 {
            if let Some(sql) = args[0].as_str() {
                let client = self.client.lock().unwrap().as_ref().map(Arc::clone);
                match client {
                    Some(client) => match run_query(&client, sql).await {
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
                    },
                    None => Err(Value::from(
                        "not connected to a database (call connect() first)",
                    )),
                }
            } else {
                Err(Value::from(format!(
                    "method \"query\" expects string, received {}",
                    &args[0]
                )))
            }
        } else {
            Err(Value::from(format!(
                "method \"query\" expects 1 arg, received {}",
                args.len()
            )))
        }
    }
}
