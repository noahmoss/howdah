use std::error::Error;

use tokio_postgres::{Client, SimpleQueryMessage};

#[derive(Debug)]
pub struct QueryResult {
    pub cols: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub async fn run_query(client: &Client, sql: &str) -> Result<QueryResult, Box<dyn Error>> {
    let messages = client.simple_query(sql).await?;

    let mut cols: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for msg in messages {
        match msg {
            SimpleQueryMessage::RowDescription(desc) => {
                cols = desc.iter().map(|col| col.name().to_string()).collect();
            }
            SimpleQueryMessage::Row(row) => {
                let mut cells: Vec<String> = Vec::new();
                for idx in 0..row.len() {
                    let cell = match row.get(idx) {
                        Some(s) => s.to_string(),
                        None => "NULL".to_string(),
                    };
                    cells.push(cell)
                }
                rows.push(cells)
            }
            _ => {}
        }
    }

    Ok(QueryResult { cols, rows })
}
