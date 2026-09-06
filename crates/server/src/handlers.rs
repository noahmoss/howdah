use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use howdah_core::run_query;
use nvim_rs::{Handler, Neovim, Value, compat::tokio::Compat};
use tokio::fs::File;
use tokio_postgres::{Client, Config, NoTls};

use crate::protocol::{decode_args, statement_result_to_msgpack};

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
            "connect" => {
                let (connection_string,): (String,) = decode_args(&name, args)?;
                self.handle_connect(&connection_string).await
            }
            "query" => {
                let (sql,): (String,) = decode_args(&name, args)?;
                self.handle_query(&sql).await
            }
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

    async fn handle_connect(&self, connection_string: &str) -> Result<Value, Value> {
        let config = build_config(connection_string).map_err(|e| {
            Value::from(format!(
                "failed to parse connection string: {}",
                error_chain(unwrap_db_error(&e))
            ))
        })?;

        let (client, connection) = config.connect(NoTls).await.map_err(|e| {
            Value::from(format!(
                "failed to connect: {}",
                error_chain(unwrap_db_error(&e))
            ))
        })?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", error_chain(&e));
            }
        });

        *self.client.lock().unwrap() = Some(Arc::new(client));

        Ok(Value::Nil)
    }

    async fn handle_query(&self, sql: &str) -> Result<Value, Value> {
        let Some(client) = self.current_client() else {
            return Err(Value::from(
                "not connected to a database (call connect() first)",
            ));
        };

        // Only server failures use the RPC error channel; SQL errors are data.
        let results = run_query(&client, sql)
            .await
            .map_err(|err| Value::from(format!("execution error: {}", error_chain(&err))))?;

        Ok(Value::Array(
            results
                .into_iter()
                .map(statement_result_to_msgpack)
                .collect(),
        ))
    }
}

fn build_config(connection_string: &str) -> Result<Config, tokio_postgres::Error> {
    let mut config: Config = connection_string.parse()?;

    // When not provided, host defaults to /tmp as the domain socket directory
    // to match default libpq behavior.
    if config.get_hosts().is_empty() {
        config.host("/tmp");
    };

    Ok(config)
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

// Skip the "db error: " prefix attached by tokio-postgres; return the
// underlying DB error directly to Neovim.
fn unwrap_db_error(err: &tokio_postgres::Error) -> &dyn Error {
    err.as_db_error().map_or(err, |db_err| db_err)
}
