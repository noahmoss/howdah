use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use howdah_core::{QueryError, QueryResult, SqlError, run_query};
use nvim_rs::{Handler, Neovim, Value, compat::tokio::Compat};
use tokio::fs::File;
use tokio_postgres::{Client, Config, NoTls};

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

        // Neovim only accepts strings on the RPC error channel, so a database
        // error travels on the Ok side, tagged like a Result: {Ok: ...} or
        // {Err: ...}. The RPC Err channel is reserved for failures of the
        // server itself.
        let (tag, payload) = match run_query(&client, sql).await {
            Ok(result) => ("Ok", query_result_to_msgpack(result)),
            Err(QueryError::Sql(info)) => ("Err", sql_error_to_msgpack(info)),
            Err(QueryError::Other(err)) => {
                return Err(Value::from(format!(
                    "execution error: {}",
                    error_chain(&err)
                )));
            }
        };

        Ok(Value::Map(vec![(Value::from(tag), payload)]))
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

/// Returns a msgpack value representing the query results, with `cols`
/// formatted as an array of strings (omitted when there is no result set),
/// `rows` as an array of arrays of strings, and `row_count` as an integer.
fn query_result_to_msgpack(result: QueryResult) -> Value {
    let QueryResult {
        rows,
        cols,
        row_count,
    } = result;
    let row_values = Value::Array(rows.into_iter().map(string_array).collect());
    let mut fields = vec![
        (Value::from("rows"), row_values),
        (Value::from("row_count"), Value::from(row_count)),
    ];
    push_some(&mut fields, "cols", cols.map(string_array));
    Value::Map(fields)
}

fn string_array(strings: Vec<String>) -> Value {
    Value::Array(strings.into_iter().map(Value::from).collect())
}

/// Appends `key: value` to a msgpack map's fields when the value is present.
fn push_some<T: Into<Value>>(fields: &mut Vec<(Value, Value)>, key: &str, value: Option<T>) {
    if let Some(value) = value {
        fields.push((Value::from(key), value.into()));
    }
}

/// Returns a msgpack map of the Postgres error fields, keyed by field name.
/// Optional fields Postgres did not send are omitted rather than sent as nil.
fn sql_error_to_msgpack(info: SqlError) -> Value {
    let SqlError {
        severity,
        code,
        message,
        detail,
        hint,
        context,
        position,
        internal_position,
        internal_query,
    } = info;

    let mut fields = vec![
        (Value::from("severity"), Value::from(severity)),
        (Value::from("code"), Value::from(code)),
        (Value::from("message"), Value::from(message)),
    ];

    push_some(&mut fields, "detail", detail);
    push_some(&mut fields, "hint", hint);
    push_some(&mut fields, "context", context);
    push_some(&mut fields, "position", position);
    push_some(&mut fields, "internal_position", internal_position);
    push_some(&mut fields, "internal_query", internal_query);

    Value::Map(fields)
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
