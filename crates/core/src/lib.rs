use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tokio_postgres::{
    Client, SimpleColumn, SimpleQueryMessage, SimpleQueryRow,
    error::{DbError, ErrorPosition},
};

#[derive(Debug, Default)]
pub struct StatementResult {
    /// None when the statement returns no result set (e.g. DDL).
    pub cols: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
    /// Rows returned or affected. None when the command tag has no row count.
    pub row_count: Option<u64>,
    /// The full command tag returned by PostgreSQL, unchanged.
    pub tag: String,
}

/// Everything one `run_query` call produced.
#[derive(Debug)]
pub struct QueryRun {
    /// One entry per statement that ran, in execution order.
    pub statements: Vec<Result<StatementResult, SqlError>>,
    /// Time for the whole run, measured client-side.
    pub elapsed: Duration,
}

#[derive(Debug)]
pub struct SqlError {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    pub context: Option<String>,
    pub position: Option<u32>,
    pub internal_position: Option<u32>,
    pub internal_query: Option<String>,
}

impl From<&DbError> for SqlError {
    fn from(err: &DbError) -> Self {
        let (position, internal_position, internal_query) = match err.position() {
            None => (None, None, None),
            Some(ErrorPosition::Original(p)) => (Some(*p), None, None),
            Some(ErrorPosition::Internal { position, query }) => {
                (None, Some(*position), Some(query.clone()))
            }
        };
        SqlError {
            severity: err.severity().to_string(),
            code: err.code().code().to_string(),
            message: err.message().to_string(),
            detail: err.detail().map(str::to_string),
            hint: err.hint().map(str::to_string),
            context: err.where_().map(str::to_string),
            position,
            internal_position,
            internal_query,
        }
    }
}

pub async fn run_query(client: &Client, sql: &str) -> Result<QueryRun, tokio_postgres::Error> {
    let started = Instant::now();
    let stream = client.simple_query_raw(sql).await?;
    tokio::pin!(stream);

    let mut statements = Vec::new();
    let mut current = StatementResult::default();

    while let Some(msg) = stream.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(err) => match err.as_db_error() {
                Some(db_error) => {
                    statements.push(Err(db_error.into()));
                    break;
                }
                None => return Err(err),
            },
        };
        match msg {
            SimpleQueryMessage::RowDescription(desc) => current.cols = Some(column_names(&desc)),
            SimpleQueryMessage::Row(row) => current.rows.push(cells(&row)),
            SimpleQueryMessage::CommandComplete(command) => {
                current.row_count = command.rows_affected();
                current.tag = command.tag().to_owned();
                statements.push(Ok(current));
                current = StatementResult::default();
            }
            // No statement ran, so there is no result to collect.
            SimpleQueryMessage::EmptyQueryResponse => {}
            _ => {}
        }
    }

    Ok(QueryRun {
        statements,
        elapsed: started.elapsed(),
    })
}

fn column_names(desc: &[SimpleColumn]) -> Vec<String> {
    desc.iter().map(|col| col.name().to_string()).collect()
}

fn cells(row: &SimpleQueryRow) -> Vec<String> {
    (0..row.len())
        .map(|i| row.get(i).unwrap_or("NULL").to_string())
        .collect()
}
