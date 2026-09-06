use howdah_core::{QueryResult, SqlError, StatementResult};
use nvim_rs::Value;

pub(crate) fn statement_result_to_msgpack(result: StatementResult) -> Value {
    let (tag, payload) = match result {
        Ok(result) => ("ok", query_result_to_msgpack(result)),
        Err(error) => ("err", sql_error_to_msgpack(error)),
    };
    Value::Map(vec![(Value::from(tag), payload)])
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

fn string_array(strings: Vec<String>) -> Value {
    Value::Array(strings.into_iter().map(Value::from).collect())
}

/// Appends `key: value` to a msgpack map's fields when the value is present.
fn push_some<T: Into<Value>>(fields: &mut Vec<(Value, Value)>, key: &str, value: Option<T>) {
    if let Some(value) = value {
        fields.push((Value::from(key), value.into()));
    }
}
