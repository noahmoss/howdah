use howdah_core::{QueryResult, SqlError, StatementResult};
use nvim_rs::Value;
use serde::de::DeserializeOwned;

pub(crate) fn decode_args<T: DeserializeOwned>(method: &str, args: Vec<Value>) -> Result<T, Value> {
    rmpv::ext::from_value(Value::Array(args))
        .map_err(|err| Value::from(format!("invalid arguments for method \"{method}\": {err}")))
}

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

#[cfg(test)]
mod tests {
    use nvim_rs::Value;

    use super::decode_args;

    #[test]
    fn decodes_one_string_argument() {
        for text in ["", "host=localhost dbname=howdah_dev", "select 'café'"] {
            let (decoded,): (String,) = decode_args("query", vec![Value::from(text)]).unwrap();
            assert_eq!(decoded, text);
        }
    }

    #[test]
    fn rejects_missing_or_extra_arguments() {
        for args in [vec![], vec![Value::from("one"), Value::from("two")]] {
            assert!(decode_args::<(String,)>("query", args).is_err());
        }
    }

    #[test]
    fn rejects_wrong_types_with_method_context() {
        for arg in [
            Value::Nil,
            Value::from(42),
            Value::from(true),
            Value::Array(vec![]),
        ] {
            let err = decode_args::<(String,)>("connect", vec![arg]).unwrap_err();
            assert!(
                err.as_str()
                    .unwrap()
                    .starts_with("invalid arguments for method \"connect\": ")
            );
        }
    }
}
