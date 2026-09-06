use tokio_postgres::{
    Client, SimpleColumn, SimpleQueryMessage, SimpleQueryRow,
    error::{DbError, ErrorPosition},
};

#[derive(Debug)]
pub struct QueryResult {
    /// None when the statement returns no result set (e.g. DDL).
    pub cols: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
    /// Rows returned or affected, per CommandComplete.
    pub row_count: u64,
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

#[derive(Debug)]
pub enum QueryError {
    Sql(SqlError),
    Other(tokio_postgres::Error),
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

impl From<tokio_postgres::Error> for QueryError {
    fn from(err: tokio_postgres::Error) -> Self {
        match err.as_db_error() {
            Some(db_error) => QueryError::Sql(db_error.into()),
            None => QueryError::Other(err),
        }
    }
}

pub async fn run_query(client: &Client, sql: &str) -> Result<QueryResult, QueryError> {
    let messages = client.simple_query(sql).await?;

    let mut cols: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row_count: u64 = 0;
    for msg in messages {
        match msg {
            SimpleQueryMessage::RowDescription(desc) => cols = Some(column_names(&desc)),
            SimpleQueryMessage::Row(row) => rows.push(cells(&row)),
            // One per statement; last wins, like cols.
            SimpleQueryMessage::CommandComplete(count) => row_count = count,
            _ => {}
        }
    }

    Ok(QueryResult {
        cols,
        rows,
        row_count,
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
