//! Rough port of parts of
//! @postgres/postgres/blob/master/src/fe_utils/psqlscan.l for splitting a SQL
//! string on statement boundaries.

use std::{iter::Peekable, str::CharIndices};

/// Identifier keywords that impact scanning behavior
#[derive(Clone, Copy, Default)]
enum Keyword {
    Create,
    Or,
    Replace,
    Function,
    Procedure,
    #[default]
    Other,
}

impl Keyword {
    fn from_identifier(identifier: &str) -> Self {
        match identifier {
            s if s.eq_ignore_ascii_case("create") => Self::Create,
            s if s.eq_ignore_ascii_case("or") => Self::Or,
            s if s.eq_ignore_ascii_case("replace") => Self::Replace,
            s if s.eq_ignore_ascii_case("function") => Self::Function,
            s if s.eq_ignore_ascii_case("procedure") => Self::Procedure,
            _ => Self::Other,
        }
    }
}

/// How a backslash inside a quoted token is treated
#[derive(Clone, Copy, PartialEq)]
enum Backslash {
    Literal,
    /// Escapes the next character (psql's `xeescape`)
    Escapes,
}

#[derive(Default)]
struct ScanState {
    paren_depth: i32,
    begin_depth: i32,
    init_idents: [Keyword; 4],
    init_idents_count: usize,
}

impl ScanState {
    /// Port of psqlscan_track_identifier
    fn track_identifier(&mut self, identifier: &str) {
        if self.paren_depth != 0 {
            return;
        }

        if self.init_idents_count == 0 {
            self.init_idents.fill(Keyword::Other);
        }
        self.record_initial_keyword(identifier);

        if self.is_create_routine() {
            if self.opens_block(identifier) {
                self.begin_depth += 1;
            } else if self.closes_block(identifier) {
                self.begin_depth -= 1;
            }
        }
    }

    fn opens_block(&self, identifier: &str) -> bool {
        identifier.eq_ignore_ascii_case("begin")
            || (identifier.eq_ignore_ascii_case("case") && self.begin_depth > 0)
    }

    fn closes_block(&self, identifier: &str) -> bool {
        identifier.eq_ignore_ascii_case("end") && self.begin_depth > 0
    }

    fn record_initial_keyword(&mut self, identifier: &str) {
        if self.init_idents_count < self.init_idents.len() {
            self.init_idents[self.init_idents_count] = Keyword::from_identifier(identifier);
            self.init_idents_count += 1;
        }
    }

    /// Does the current input match CREATE [OR REPLACE] {FUNCTION|PROCEDURE}?
    fn is_create_routine(&self) -> bool {
        use Keyword::{Create, Function, Or, Procedure, Replace};

        matches!(
            self.init_idents,
            [Create, Function | Procedure, _, _] | [Create, Or, Replace, Function | Procedure]
        )
    }
}

fn split_statements(sql: &str) -> Vec<&str> {
    let mut chars = sql.char_indices().peekable();

    let mut statements = Vec::new();
    let mut statement_start = 0;

    let mut state = ScanState::default();

    while let Some((i, c)) = chars.next() {
        let next = chars.peek().map(|&(_, n)| n);

        match (c, next) {
            ('\'', _) => skip_quoted(&mut chars, '\'', Backslash::Literal),
            ('"', _) => skip_quoted(&mut chars, '"', Backslash::Literal),
            // Escape string (psql's `xestart`)
            ('e' | 'E', Some('\'')) => {
                chars.next(); // skip the opening quote
                skip_quoted(&mut chars, '\'', Backslash::Escapes);
            }
            ('(', _) => {
                state.paren_depth += 1;
            }
            (')', _) => {
                if state.paren_depth > 0 {
                    state.paren_depth -= 1;
                }
            }
            (';', _) if state.paren_depth == 0 && state.begin_depth == 0 => {
                statements.push(&sql[statement_start..i + 1]);
                statement_start = i + 1;
                state.init_idents_count = 0;
            }
            (c, _) if ident_start(c) => {
                while chars.next_if(|&(_, n)| ident_cont(n)).is_some() {}
                let end = chars.peek().map_or(sql.len(), |&(i_next, _)| i_next);

                let identifier = &sql[i..end];
                state.track_identifier(identifier);
            }
            _ => {}
        }
    }

    let closing_statement = &sql[statement_start..];
    if !closing_statement.is_empty() {
        statements.push(closing_statement)
    }

    statements
}

/// Start of an identifier: [A-Za-z\200-\377_]
fn ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || !c.is_ascii() || c == '_'
}

/// Continuation of an identifier: [A-Za-z\200-\377_0-9\$]
fn ident_cont(c: char) -> bool {
    ident_start(c) || c.is_ascii_digit() || c == '$'
}

/// Consume the rest of a quoted token through its closing delimiter.
/// Unterminated input consumes to the end.
fn skip_quoted(chars: &mut Peekable<CharIndices>, delimiter: char, backslash: Backslash) {
    while let Some((_, c)) = chars.next() {
        match c {
            '\\' if backslash == Backslash::Escapes => {
                chars.next();
            }
            c if c == delimiter => {
                // A doubled delimiter is escaped (psql's `xqdouble` / `xddouble`)
                let escaped = chars.next_if(|&(_, next)| next == delimiter).is_some();
                if !escaped {
                    return;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::split_statements;

    /// `statement` stays one statement, and a boundary follows right after it.
    #[track_caller]
    fn assert_not_split(statement: &str) {
        let sql = format!("{statement}SELECT 2;");
        assert_eq!(
            split_statements(&sql),
            [statement, "SELECT 2;"],
            "{statement}"
        );
    }

    #[test]
    fn empty_input_has_no_statements() {
        assert!(split_statements("").is_empty());
    }

    #[test]
    fn splits_at_semicolons_and_keeps_unterminated_tail() {
        assert_eq!(
            split_statements("SELECT 1;SELECT 2;SELECT 3"),
            ["SELECT 1;", "SELECT 2;", "SELECT 3"]
        );
        assert_eq!(split_statements("SELECT 1;"), ["SELECT 1;"]);
        assert_eq!(split_statements("SELECT 1"), ["SELECT 1"]);
    }

    #[test]
    fn preserves_whitespace_in_statement_slices() {
        assert_eq!(
            split_statements("  SELECT 1;\n\tSELECT 2;\n"),
            ["  SELECT 1;", "\n\tSELECT 2;", "\n"]
        );
    }

    #[test]
    fn preserves_empty_statements() {
        assert_eq!(split_statements(";SELECT 1;;"), [";", "SELECT 1;", ";"]);
    }

    #[test]
    fn splits_after_multibyte_identifiers() {
        assert_eq!(
            split_statements("SELECT café;SELECT 日本語"),
            ["SELECT café;", "SELECT 日本語"]
        );
    }

    #[test]
    fn waits_for_all_parentheses_to_close() {
        // Deliberately invalid SQL: the scanner finds boundaries, not syntax errors.
        assert_eq!(
            split_statements("SELECT (1; (2; 3); 4);SELECT 5;"),
            ["SELECT (1; (2; 3); 4);", "SELECT 5;"]
        );
        assert_eq!(
            split_statements("SELECT (1;SELECT 2;"),
            ["SELECT (1;SELECT 2;"]
        );
    }

    #[test]
    fn unmatched_closing_parenthesis_does_not_hide_boundaries() {
        assert_not_split("SELECT 1);");
    }

    #[test]
    fn transaction_begin_does_not_open_a_routine_body() {
        assert_eq!(
            split_statements("BEGIN;INSERT INTO t VALUES (1);COMMIT;"),
            ["BEGIN;", "INSERT INTO t VALUES (1);", "COMMIT;"]
        );
    }

    #[test]
    fn keeps_empty_statements_inside_atomic_body() {
        assert_not_split(
            "CREATE FUNCTION f() RETURNS boolean\nBEGIN ATOMIC\n;;RETURN false;;\nEND;",
        );
    }

    #[test]
    fn keeps_multiple_statements_inside_atomic_body() {
        assert_not_split(
            "CREATE FUNCTION f() RETURNS boolean\nBEGIN ATOMIC\nSELECT 1;\nSELECT false;\nEND;",
        );
    }

    #[test]
    fn case_end_does_not_close_atomic_body() {
        assert_not_split(
            "CREATE FUNCTION f(x int) RETURNS boolean LANGUAGE SQL
BEGIN ATOMIC
    SELECT CASE WHEN x % 2 = 0 THEN true ELSE false END;
END;",
        );
    }

    #[test]
    fn recognizes_all_routine_prefixes_case_insensitively() {
        for prefix in [
            "CREATE FUNCTION",
            "CREATE PROCEDURE",
            "CREATE OR REPLACE FUNCTION",
            "CREATE OR REPLACE PROCEDURE",
            "cReAtE oR rEpLaCe fUnCtIoN",
        ] {
            assert_not_split(&format!(
                "{prefix} f() LANGUAGE SQL bEgIn ATOMIC SELECT 1; eNd;"
            ));
        }
    }

    #[test]
    fn requires_routine_keywords_at_start_of_statement() {
        for statement in [
            "SELECT CREATE FUNCTION f BEGIN;",
            "CREATE TABLE f BEGIN;",
            "CREATE OR FUNCTION f BEGIN;",
            "CREATE REPLACE FUNCTION f BEGIN;",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn keyword_prefixes_in_identifiers_do_not_open_blocks() {
        for identifier in ["beginning", "begin_", "begin1", "begin$tag", "beginé"] {
            assert_not_split(&format!(
                "CREATE FUNCTION f() RETURNS int RETURN {identifier};"
            ));
        }
    }

    #[test]
    fn ignores_block_keywords_inside_parentheses() {
        assert_not_split(
            "CREATE FUNCTION f(begin int) RETURNS int BEGIN ATOMIC SELECT (end); SELECT 2; END;",
        );
    }

    #[test]
    fn tracks_nested_case_expressions() {
        assert_not_split(
            "CREATE FUNCTION f() RETURNS int BEGIN ATOMIC
SELECT CASE WHEN true THEN CASE WHEN false THEN 1 ELSE 2 END ELSE 3 END;
SELECT 4;
END;",
        );
    }

    #[test]
    fn case_outside_atomic_body_does_not_change_block_depth() {
        assert_not_split(
            "CREATE FUNCTION f() RETURNS int RETURN CASE WHEN true THEN 1 ELSE 2 END;",
        );
    }

    #[test]
    fn resets_routine_tracking_between_statements() {
        let routine = "CREATE OR REPLACE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1; END;";
        let procedure = "CREATE PROCEDURE p() LANGUAGE SQL BEGIN ATOMIC SELECT 2; END;";
        let sql = format!("{routine}BEGIN;SELECT 3;COMMIT;{procedure}");
        assert_eq!(
            split_statements(&sql),
            [routine, "BEGIN;", "SELECT 3;", "COMMIT;", procedure]
        );
    }

    #[test]
    fn keeps_unfinished_routine_as_one_tail() {
        let sql = "CREATE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1; SELECT 2;";
        assert_eq!(split_statements(sql), [sql]);
    }

    #[test]
    fn ignores_semicolons_inside_single_quotes() {
        assert_not_split("SELECT ';';");
    }

    #[test]
    fn doubled_single_quotes_do_not_close_string() {
        for statement in [
            "SELECT 'it''s; fine';",
            "SELECT '';",
            "SELECT '''';",
            "SELECT ''';''';",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn ignores_parentheses_inside_single_quotes() {
        assert_eq!(
            split_statements("SELECT '(';SELECT ')';SELECT 3;"),
            ["SELECT '(';", "SELECT ')';", "SELECT 3;"]
        );
    }

    #[test]
    fn ignores_block_keywords_inside_single_quotes() {
        let routine = "CREATE FUNCTION f() RETURNS text BEGIN ATOMIC SELECT 'end;'; END;";
        let sql = format!("{routine}SELECT 'begin';SELECT 3;");
        assert_eq!(
            split_statements(&sql),
            [routine, "SELECT 'begin';", "SELECT 3;"]
        );
    }

    #[test]
    fn prefixed_string_constants_are_quoted() {
        assert_not_split("SELECT x'1;', B'0;', n'a;', U&'d;';");
    }

    #[test]
    fn keeps_unterminated_single_quote_as_one_tail() {
        let sql = "SELECT 'abc; SELECT 2;";
        assert_eq!(split_statements(sql), [sql]);
    }

    #[test]
    fn ignores_semicolons_and_parentheses_inside_double_quotes() {
        for statement in [
            r#"SELECT 1 AS "a;b";"#,
            r#"SELECT 1 AS "(";"#,
            r#"SELECT 1 AS ")";"#,
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn doubled_double_quotes_do_not_close_identifier() {
        for statement in [r#"SELECT 1 AS "a""b;";"#, r#"SELECT 1 AS """;""";"#] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn quoted_identifiers_are_not_block_keywords() {
        let first = r#"CREATE FUNCTION "begin"() RETURNS int RETURN 1;"#;
        let routine = r#"CREATE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT "end" FROM t; END;"#;
        let sql = format!("{first}{routine}SELECT 3;");
        assert_eq!(split_statements(&sql), [first, routine, "SELECT 3;"]);
    }

    #[test]
    fn each_quote_kind_ignores_the_other() {
        for statement in [r#"SELECT 1 AS "it's";"#, r#"SELECT '"';"#] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn keeps_unterminated_double_quote_as_one_tail() {
        let sql = r#"SELECT 1 AS "abc; SELECT 2;"#;
        assert_eq!(split_statements(sql), [sql]);
    }

    #[test]
    fn backslash_escapes_do_not_close_escape_string() {
        for statement in [
            r"SELECT E'it\'s; fine';",
            r"SELECT e'\';';",
            r"SELECT E'\\';",
            r"SELECT E'\\\';';",
            r"SELECT E'it''s;';",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn e_inside_identifier_does_not_start_escape_string() {
        for statement in [r"SELECT text'a\';", "SELECT e;"] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn keeps_unterminated_escape_string_as_one_tail() {
        let sql = r"SELECT E'abc\'; SELECT 2;";
        assert_eq!(split_statements(sql), [sql]);
    }
}
