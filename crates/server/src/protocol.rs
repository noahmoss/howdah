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
/// `rows` as an array of arrays of strings, the command `tag`, and an optional
/// integer `row_count`.
fn query_result_to_msgpack(result: QueryResult) -> Value {
    let QueryResult {
        rows,
        cols,
        row_count,
        tag,
    } = result;
    let row_values = Value::Array(rows.into_iter().map(string_array).collect());
    let mut fields = vec![
        (Value::from("rows"), row_values),
        (Value::from("tag"), Value::from(tag)),
    ];
    push_some(&mut fields, "row_count", row_count);
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
    use howdah_core::{QueryResult, SqlError};
    use nvim_rs::Value;

    use super::{decode_args, statement_result_to_msgpack};

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
    #[test]
    fn preserves_result_columns_rows_and_count() {
        let result = QueryResult {
            cols: Some(vec!["name".into()]),
            rows: vec![vec!["café".into()]],
            row_count: Some(1),
            tag: "SELECT 1".into(),
        };
        assert_eq!(
            statement_result_to_msgpack(Ok(result)),
            Value::Map(vec![(
                Value::from("ok"),
                Value::Map(vec![
                    (
                        Value::from("rows"),
                        Value::Array(vec![Value::Array(vec![Value::from("café")])])
                    ),
                    (Value::from("tag"), Value::from("SELECT 1")),
                    (Value::from("row_count"), Value::from(1)),
                    (Value::from("cols"), Value::Array(vec![Value::from("name")])),
                ])
            )])
        );
    }

    #[test]
    fn distinguishes_empty_result_sets_from_commands() {
        for (cols, tag, count) in [
            (None, "UPDATE 2", 2),
            (Some(vec!["id".into()]), "SELECT 0", 0),
        ] {
            let result = QueryResult {
                cols: cols.clone(),
                rows: vec![],
                row_count: Some(count),
                tag: tag.into(),
            };
            let value = statement_result_to_msgpack(Ok(result));
            let payload = &value.as_map().unwrap()[0].1;
            let fields = payload.as_map().unwrap();
            let columns = fields.iter().find(|(key, _)| key.as_str() == Some("cols"));
            assert_eq!(columns.is_some(), cols.is_some());
            assert!(fields.contains(&(Value::from("rows"), Value::Array(vec![]))));
            assert!(fields.contains(&(Value::from("row_count"), Value::from(count))));
            assert!(fields.iter().all(|(_, value)| !value.is_nil()));
        }
    }

    #[test]
    fn omits_absent_counts_but_preserves_zero() {
        for (tag, count) in [("CREATE TABLE", None), ("UPDATE 0", Some(0))] {
            let result = QueryResult {
                tag: tag.into(),
                row_count: count,
                ..QueryResult::default()
            };
            let value = statement_result_to_msgpack(Ok(result));
            let fields = value.as_map().unwrap()[0].1.as_map().unwrap();
            assert!(fields.contains(&(Value::from("tag"), Value::from(tag))));
            let row_count = fields
                .iter()
                .find(|(key, _)| key.as_str() == Some("row_count"));
            assert_eq!(row_count.map(|(_, value)| value.as_u64().unwrap()), count);
            assert!(fields.iter().all(|(_, value)| !value.is_nil()));
        }
    }

    #[test]
    fn preserves_error_fields_and_omits_missing_fields() {
        for with_context in [false, true] {
            let error = SqlError {
                severity: "ERROR".into(),
                code: "42601".into(),
                message: "bad syntax".into(),
                detail: with_context.then(|| "first\nsecond".into()),
                hint: with_context.then(|| "try again".into()),
                context: with_context.then(|| "function f".into()),
                position: (!with_context).then_some(12),
                internal_position: with_context.then_some(5),
                internal_query: with_context.then(|| "select x".into()),
            };
            let value = statement_result_to_msgpack(Err(error));
            let envelope = value.as_map().unwrap();
            assert_eq!(envelope.len(), 1);
            assert_eq!(envelope[0].0.as_str(), Some("err"));
            let fields: std::collections::BTreeMap<_, _> = envelope[0]
                .1
                .as_map()
                .unwrap()
                .iter()
                .map(|(key, value)| (key.as_str().unwrap(), value.clone()))
                .collect();
            let mut expected = std::collections::BTreeMap::from([
                ("severity", Value::from("ERROR")),
                ("code", Value::from("42601")),
                ("message", Value::from("bad syntax")),
            ]);
            if with_context {
                expected.extend([
                    ("detail", Value::from("first\nsecond")),
                    ("hint", Value::from("try again")),
                    ("context", Value::from("function f")),
                    ("internal_position", Value::from(5)),
                    ("internal_query", Value::from("select x")),
                ]);
            } else {
                expected.insert("position", Value::from(12));
            }
            assert_eq!(fields, expected);
        }
    }
}
