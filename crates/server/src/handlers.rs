use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use howdah_core::{QueryResult, run_query};
use nvim_rs::{Handler, Neovim, Value, compat::tokio::Compat};
use tokio::fs::File;
use tokio_postgres::{Client, NoTls};

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
            "connect" => self.handle_connect(args).await,
            "query" => self.handle_query(args).await,
            _ => Err(Value::from(format!("unknown method: {}", name))),
        }
    }
}

impl NeovimHandler {
    /// Returns the current database client, if one is connected.
    ///
    /// The lock serializes access to the client slot, not use of the client
    /// itself, so we return a cloned handle and release the lock immediately.
    fn current_client(&self) -> Option<Arc<Client>> {
        self.client.lock().unwrap().as_ref().map(Arc::clone)
    }

    async fn handle_connect(&self, args: Vec<Value>) -> Result<Value, Value> {
        if args.len() != 1 {
            return Err(Value::from(format!(
                "method \"connect\" expects 1 arg, received {}",
                args.len()
            )));
        }

        let Some(connection_string) = args[0].as_str() else {
            return Err(Value::from(format!(
                "method \"connect\" expects string, received {}",
                &args[0]
            )));
        };

        let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
            .await
            .map_err(|e| Value::from(format!("connection error: {}", error_chain(&e))))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", error_chain(&e));
            }
        });

        *self.client.lock().unwrap() = Some(Arc::new(client));

        Ok(Value::Nil)
    }

    async fn handle_query(&self, args: Vec<Value>) -> Result<Value, Value> {
        if args.len() != 1 {
            return Err(Value::from(format!(
                "method \"query\" expects 1 arg, received {}",
                args.len()
            )));
        }

        let Some(sql) = args[0].as_str() else {
            return Err(Value::from(format!(
                "method \"query\" expects string, received {}",
                &args[0]
            )));
        };

        let Some(client) = self.current_client() else {
            return Err(Value::from(
                "not connected to a database (call connect() first)",
            ));
        };

        match run_query(&client, sql).await {
            Ok(result) => Ok(query_result_to_msgpack(result)),
            Err(err) => {
                let chain = error_chain(err.as_ref());
                Err(Value::from(format!("execution error: {}", chain)))
            }
        }
    }
}

/// Returns a msgpack value representing the query results, with `cols`
/// formatted as an array of strings, and `rows` formatted as an array of arrays
/// of strings.
fn query_result_to_msgpack(result: QueryResult) -> Value {
    let QueryResult { rows, cols } = result;
    let col_value = Value::Array(cols.into_iter().map(Value::from).collect());
    let row_values = Value::Array(
        rows.into_iter()
            .map(|row| Value::Array(row.into_iter().map(Value::from).collect()))
            .collect(),
    );
    Value::Map(vec![
        (Value::from("cols"), col_value),
        (Value::from("rows"), row_values),
    ])
}

/// Returns an error formatted with its chain of causes, one per line
pub(crate) fn error_chain(err: &dyn Error) -> String {
    let mut err_text = format!("{}", err);
    let mut source = err.source();
    while let Some(e) = source {
        err_text.push_str(&format!("\nCaused by: {}", e));
        source = e.source()
    }
    err_text
}
