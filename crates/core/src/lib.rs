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

    // build_results(rows)
    Ok(QueryResult { cols, rows })
}

// Unused prototype of extended query protocol (maybe useful later)

// pub fn build_results(rows: Vec<Row>) -> Result<QueryResult, Box<dyn Error>> {
//     let cols: Vec<String> = match rows.first() {
//         Some(row) => row
//             .columns()
//             .iter()
//             .map(|col| col.name().to_string())
//             .collect(),
//         None => Vec::new(),
//     };
//
//     let mut stringified_rows: Vec<Vec<String>> = Vec::new();
//     for row in rows {
//         let row_str = stringify_row(&row)?;
//         stringified_rows.push(row_str)
//     }
//
//     Ok(QueryResult {
//         cols,
//         rows: stringified_rows,
//     })
// }
//
// fn get_cell<'a, T>(row: &'a Row, idx: usize) -> Result<String, Box<dyn Error>>
// where
//     T: FromSql<'a> + Display,
// {
//     let val: Option<T> = row.try_get(idx)?;
//     let val_str = match val {
//         Some(v) => v.to_string(),
//         None => "NULL".to_string(),
//     };
//     Ok(val_str)
// }
//
// pub fn stringify_row(row: &Row) -> Result<Vec<String>, Box<dyn Error>> {
//     let mut vals: Vec<String> = Vec::new();
//     for (idx, col) in row.columns().iter().enumerate() {
//         let col_type = col.type_();
//         let val: String = match *col_type {
//             Type::INT4 => get_cell::<i32>(row, idx)?,
//             Type::TEXT => get_cell::<String>(row, idx)?,
//             _ => {
//                 format!("<{}>", col_type.name().to_uppercase())
//             }
//         };
//         vals.push(val)
//     }
//     Ok(vals)
// }
