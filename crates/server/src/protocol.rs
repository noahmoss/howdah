use howdah_core::{QueryRun, SqlError, StatementResult};
use nvim_rs::Value;
use serde::de::DeserializeOwned;

pub(crate) fn decode_args<T: DeserializeOwned>(method: &str, args: Vec<Value>) -> Result<T, Value> {
    rmpv::ext::from_value(Value::Array(args))
        .map_err(|err| Value::from(format!("invalid arguments for method \"{method}\": {err}")))
}

/// Builds the wire map for a whole run: `statements` in execution order and
/// `elapsed` in microseconds.
pub(crate) fn query_run_to_msgpack(run: QueryRun) -> Value {
    let QueryRun {
        statements,
        elapsed,
    } = run;
    let statement_values = statements.into_iter().map(statement_to_msgpack).collect();

    // msgpack integers are at most 64 bits. u64 microseconds covers ~584,000
    // years, so the narrowing cannot overflow in practice.
    let elapsed_micros = Value::from(elapsed.as_micros() as u64);

    map_of([
        ("statements", Some(Value::Array(statement_values))),
        ("elapsed", Some(elapsed_micros)),
    ])
}

fn statement_to_msgpack(result: Result<StatementResult, SqlError>) -> Value {
    let (tag, payload) = match result {
        Ok(result) => ("ok", statement_result_to_msgpack(result)),
        Err(error) => ("err", sql_error_to_msgpack(error)),
    };
    Value::Map(vec![(Value::from(tag), payload)])
}

/// Builds the wire map for one statement's result, keyed by the
/// `StatementResult` field names.
fn statement_result_to_msgpack(result: StatementResult) -> Value {
    let StatementResult {
        rows,
        cols,
        row_count,
        tag,
    } = result;
    let row_values = Value::Array(rows.into_iter().map(string_array).collect());

    map_of([
        ("rows", Some(row_values)),
        ("tag", Some(Value::from(tag))),
        ("row_count", row_count.map(Value::from)),
        ("cols", cols.map(string_array)),
    ])
}

/// Builds the wire map of the Postgres error fields, keyed by field name.
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

    map_of([
        ("severity", Some(Value::from(severity))),
        ("code", Some(Value::from(code))),
        ("message", Some(Value::from(message))),
        ("detail", detail.map(Value::from)),
        ("hint", hint.map(Value::from)),
        ("context", context.map(Value::from)),
        ("position", position.map(Value::from)),
        ("internal_position", internal_position.map(Value::from)),
        ("internal_query", internal_query.map(Value::from)),
    ])
}

/// Builds a msgpack map from `(key, value)` pairs. Absent values are omitted
/// rather than sent as nil: Lua cannot tell a missing key from nil anyway, and
/// a `vim.NIL` sentinel would force a presence check on every read.
fn map_of<'a>(fields: impl IntoIterator<Item = (&'a str, Option<Value>)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .filter_map(|(key, value)| value.map(|value| (Value::from(key), value)))
            .collect(),
    )
}

fn string_array(strings: Vec<String>) -> Value {
    Value::Array(strings.into_iter().map(Value::from).collect())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use howdah_core::{QueryRun, SqlError, StatementResult};
    use nvim_rs::Value;

    use super::{decode_args, query_run_to_msgpack, statement_to_msgpack};

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
    fn wraps_ordered_statements_with_elapsed_micros() {
        let statement = |tag: &str| StatementResult {
            tag: tag.into(),
            ..StatementResult::default()
        };
        let run = QueryRun {
            statements: vec![Ok(statement("SELECT 1")), Ok(statement("SELECT 2"))],
            elapsed: Duration::from_micros(1500),
        };
        assert_eq!(
            query_run_to_msgpack(run),
            Value::Map(vec![
                (
                    Value::from("statements"),
                    Value::Array(vec![
                        statement_to_msgpack(Ok(statement("SELECT 1"))),
                        statement_to_msgpack(Ok(statement("SELECT 2"))),
                    ])
                ),
                (Value::from("elapsed"), Value::from(1500)),
            ])
        );
    }

    #[test]
    fn preserves_result_columns_rows_and_count() {
        let result = StatementResult {
            cols: Some(vec!["name".into()]),
            rows: vec![vec!["café".into()]],
            row_count: Some(1),
            tag: "SELECT 1".into(),
        };
        assert_eq!(
            statement_to_msgpack(Ok(result)),
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
            let result = StatementResult {
                cols: cols.clone(),
                row_count: Some(count),
                tag: tag.into(),
                ..StatementResult::default()
            };
            let value = statement_to_msgpack(Ok(result));
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
            let result = StatementResult {
                tag: tag.into(),
                row_count: count,
                ..StatementResult::default()
            };
            let value = statement_to_msgpack(Ok(result));
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
            let value = statement_to_msgpack(Err(error));
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
