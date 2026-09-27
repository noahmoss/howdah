//! Splits SQL into statements. A rough port of parts of psql's lexer,
//! `src/fe_utils/psqlscan.l` in the PostgreSQL source.

use std::{iter::Peekable, str::CharIndices};

/// Keywords that identify a `CREATE [OR REPLACE] {FUNCTION | PROCEDURE}`
/// statement.
#[derive(Clone, Copy)]
enum Keyword {
    Create,
    Or,
    Replace,
    Function,
    Procedure,
    /// Any other identifier.
    Other,
}

impl Keyword {
    /// Classifies an identifier, ignoring ASCII case.
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

/// How a backslash inside a quoted token is treated.
#[derive(Clone, Copy, PartialEq)]
enum Backslash {
    /// An ordinary character.
    Literal,
    /// Escapes the next character, so `\'` doesn't close the string.
    Escapes,
}

/// Nesting and keyword state that decides whether a `;` ends the current
/// statement.
#[derive(Default)]
struct ScanState {
    /// Open parentheses.
    paren_depth: u32,
    /// Open `BEGIN` and `CASE` blocks in a routine definition. Nonzero means
    /// we're inside a `BEGIN ATOMIC ... END` body.
    block_depth: u32,
    /// Keywords of the current statement's first identifiers, used to
    /// recognize a routine definition.
    leading_keywords: Vec<Keyword>,
}

impl ScanState {
    /// Long enough for the longest prefix, `CREATE OR REPLACE FUNCTION`.
    const MAX_LEADING_KEYWORDS: usize = 4;

    fn open_paren(&mut self) {
        self.paren_depth += 1;
    }

    /// Ignores an unmatched `)` rather than going negative.
    fn close_paren(&mut self) {
        self.paren_depth = self.paren_depth.saturating_sub(1);
    }

    /// Whether a `;` here ends the statement.
    fn at_boundary(&self) -> bool {
        self.paren_depth == 0 && self.block_depth == 0
    }

    /// Resets per-statement keyword tracking after a boundary.
    fn end_statement(&mut self) {
        self.leading_keywords.clear();
    }

    /// Tracks `BEGIN ATOMIC ... END` routine bodies, whose `;`s don't end the
    /// statement:
    ///
    /// ```sql
    /// CREATE FUNCTION f() RETURNS int BEGIN ATOMIC
    ///     SELECT CASE WHEN true THEN 1 END;  -- not a boundary
    /// END;                                   -- boundary
    /// ```
    ///
    /// - `BEGIN` only opens a block in a routine definition, so a transaction's
    ///   `BEGIN;` is still a boundary.
    /// - Inside a block, `CASE` opens one too, so its `END` doesn't close the
    ///   body.
    /// - Identifiers inside parentheses are ignored, so a parameter named
    ///   `begin` doesn't open a block.
    ///
    /// Port of `psqlscan_track_identifier`.
    fn track_identifier(&mut self, identifier: &str) {
        if self.paren_depth != 0 {
            return;
        }

        self.record_leading_keyword(identifier);

        if self.is_create_routine() {
            if self.opens_block(identifier) {
                self.block_depth += 1;
            } else if self.closes_block(identifier) {
                self.block_depth -= 1;
            }
        }
    }

    /// `BEGIN`, or `CASE` inside an open block.
    fn opens_block(&self, identifier: &str) -> bool {
        identifier.eq_ignore_ascii_case("begin")
            || (identifier.eq_ignore_ascii_case("case") && self.block_depth > 0)
    }

    /// `END` of an open block.
    fn closes_block(&self, identifier: &str) -> bool {
        identifier.eq_ignore_ascii_case("end") && self.block_depth > 0
    }

    /// Stores the identifier's keyword if it's among the statement's first
    /// few.
    fn record_leading_keyword(&mut self, identifier: &str) {
        if self.leading_keywords.len() < Self::MAX_LEADING_KEYWORDS {
            self.leading_keywords
                .push(Keyword::from_identifier(identifier));
        }
    }

    /// Whether the statement starts with
    /// `CREATE [OR REPLACE] {FUNCTION | PROCEDURE}`.
    fn is_create_routine(&self) -> bool {
        use Keyword::{Create, Function, Or, Procedure, Replace};

        matches!(
            self.leading_keywords.as_slice(),
            [Create, Function | Procedure, ..] | [Create, Or, Replace, Function | Procedure, ..]
        )
    }
}

/// One statement found by [`split_statements`].
#[derive(Debug)]
struct Statement<'a> {
    /// Byte offset of `text` in the original SQL.
    start: usize,
    text: &'a str,
    /// False when `text` is only whitespace, comments, and the closing `;`.
    has_content: bool,
}

/// Splits `sql` into statements the way psql does, without parsing it.
///
/// A `;` ends a statement unless it's inside quotes, a comment, parentheses,
/// or a `BEGIN ATOMIC` routine body. Each statement keeps its `;` and
/// surrounding whitespace, so the texts concatenate back to `sql`. Text
/// after the last `;` becomes a final statement.
fn split_statements(sql: &str) -> Vec<Statement<'_>> {
    let mut chars = sql.char_indices().peekable();
    let mut statements = Vec::new();
    let mut statement_start = 0;
    let mut has_content = false;
    let mut state = ScanState::default();

    while let Some((i, c)) = chars.next() {
        let next = chars.peek().map(|&(_, n)| n);

        let is_content = match (c, next) {
            ('\'', _) => {
                skip_quoted(&mut chars, '\'', Backslash::Literal);
                true
            }
            ('"', _) => {
                skip_quoted(&mut chars, '"', Backslash::Literal);
                true
            }
            // Escape string
            ('e' | 'E', Some('\'')) => {
                chars.next();
                skip_quoted(&mut chars, '\'', Backslash::Escapes);
                true
            }
            ('$', _) => {
                skip_dollar_quoted(&mut chars, sql, i);
                true
            }
            ('-', Some('-')) => {
                chars.next();
                skip_line_comment(&mut chars);
                false
            }
            ('/', Some('*')) => {
                chars.next();
                skip_block_comment(&mut chars);
                false
            }
            ('(', _) => {
                state.open_paren();
                true
            }
            (')', _) => {
                state.close_paren();
                true
            }
            (';', _) if state.at_boundary() => {
                statements.push(Statement {
                    start: statement_start,
                    text: &sql[statement_start..i + 1],
                    has_content,
                });
                statement_start = i + 1;
                has_content = false;
                state.end_statement();
                false
            }
            _ if is_ident_start(c) => {
                let identifier = leading_identifier(&sql[i..]);
                skip_to(&mut chars, i + identifier.len());
                state.track_identifier(identifier);
                true
            }
            _ => !c.is_whitespace(),
        };

        has_content |= is_content;
    }

    let tail = &sql[statement_start..];
    if !tail.is_empty() {
        statements.push(Statement {
            start: statement_start,
            text: tail,
            has_content,
        });
    }

    statements
}

/// Whether `c` can start an unquoted identifier: an ASCII letter, `_`, or any
/// non-ASCII character.
fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || !c.is_ascii() || c == '_'
}

/// Whether `c` can appear in an unquoted identifier after the first
/// character: anything that can start one, plus digits and `$`.
fn is_ident_char(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit() || c == '$'
}

/// The unquoted identifier at the start of `s`.
fn leading_identifier(s: &str) -> &str {
    let len = s.find(|c: char| !is_ident_char(c)).unwrap_or(s.len());
    &s[..len]
}

/// Consumes the rest of a quoted token through its closing `delimiter`.
/// Unterminated input is consumed to the end.
fn skip_quoted(chars: &mut Peekable<CharIndices>, delimiter: char, backslash: Backslash) {
    while let Some((_, c)) = chars.next() {
        match c {
            '\\' if backslash == Backslash::Escapes => {
                chars.next();
            }
            c if c == delimiter => {
                // A doubled delimiter is escaped
                let escaped = chars.next_if(|&(_, n)| n == delimiter).is_some();
                if !escaped {
                    return;
                }
            }
            _ => {}
        }
    }
}

/// Consumes the rest of a dollar-quoted string if the `$` at byte `start` opens
/// one. Does nothing for a `$` that doesn't, like the parameter `$1`.
fn skip_dollar_quoted(chars: &mut Peekable<CharIndices>, sql: &str, start: usize) {
    if let Some(len) = dollar_quote_len(&sql[start..]) {
        skip_to(chars, start + len);
    }
}

/// If `s` starts with a dollar-quoted string (`$$...$$` or `$tag$...$tag$`),
/// returns its length in bytes, including both delimiters. Unterminated input
/// runs to the end.
fn dollar_quote_len(s: &str) -> Option<usize> {
    let (tag, _) = s.strip_prefix('$')?.split_once('$')?;
    // A tag follows identifier rules
    let valid_tag =
        tag.chars().all(is_ident_char) && !tag.starts_with(|c: char| c.is_ascii_digit());
    if !valid_tag {
        return None;
    }

    let delimiter = format!("${tag}$");
    let after_open = &s[delimiter.len()..];
    let len = match after_open.find(&delimiter) {
        Some(content_len) => delimiter.len() + content_len + delimiter.len(),
        None => s.len(), // unterminated
    };
    Some(len)
}

/// Advances `chars` to byte offset `end`.
fn skip_to(chars: &mut Peekable<CharIndices>, end: usize) {
    while chars.next_if(|&(j, _)| j < end).is_some() {}
}

/// Consumes the rest of a `--` comment, up to but not including the newline.
fn skip_line_comment(chars: &mut Peekable<CharIndices>) {
    while chars.next_if(|&(_, n)| !matches!(n, '\n' | '\r')).is_some() {}
}

/// Consumes the rest of a `/* */` comment, which can nest.
/// Unterminated input is consumed to the end.
fn skip_block_comment(chars: &mut Peekable<CharIndices>) {
    let mut depth = 1;
    while let Some((_, c)) = chars.next() {
        let next = chars.peek().map(|&(_, n)| n);

        match (c, next) {
            ('/', Some('*')) => {
                chars.next();
                depth += 1;
            }
            ('*', Some('/')) => {
                chars.next();
                depth -= 1;
                if depth == 0 {
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

    /// Just the text of each statement.
    fn statement_texts(sql: &str) -> Vec<&str> {
        split_statements(sql).iter().map(|s| s.text).collect()
    }

    /// `statement` stays one statement, and a boundary follows right after it.
    #[track_caller]
    fn assert_not_split(statement: &str) {
        let sql = format!("{statement}SELECT 2;");
        assert_eq!(
            statement_texts(&sql),
            [statement, "SELECT 2;"],
            "{statement}"
        );
    }

    /// Just the `has_content` of each statement.
    fn content_flags(sql: &str) -> Vec<bool> {
        split_statements(sql)
            .iter()
            .map(|s| s.has_content)
            .collect()
    }

    #[test]
    fn empty_input_has_no_statements() {
        assert!(statement_texts("").is_empty());
    }

    #[test]
    fn splits_at_semicolons_and_keeps_unterminated_tail() {
        assert_eq!(
            statement_texts("SELECT 1;SELECT 2;SELECT 3"),
            ["SELECT 1;", "SELECT 2;", "SELECT 3"]
        );
        assert_eq!(statement_texts("SELECT 1;"), ["SELECT 1;"]);
        assert_eq!(statement_texts("SELECT 1"), ["SELECT 1"]);
    }

    #[test]
    fn preserves_whitespace_in_statement_slices() {
        assert_eq!(
            statement_texts("  SELECT 1;\n\tSELECT 2;\n"),
            ["  SELECT 1;", "\n\tSELECT 2;", "\n"]
        );
    }

    #[test]
    fn preserves_empty_statements() {
        assert_eq!(statement_texts(";SELECT 1;;"), [";", "SELECT 1;", ";"]);
    }

    #[test]
    fn whitespace_comments_and_semicolons_are_not_content() {
        for sql in [
            ";", " \n\t;", "-- x\n;", "/* x */;", "-- x", "/* x */", " \n\t",
        ] {
            assert_eq!(content_flags(sql), [false], "{sql:?}");
        }
    }

    #[test]
    fn any_other_token_is_content() {
        for sql in [
            "SELECT 1;",
            "1;",
            "+;",
            "();",
            "'';",
            "E'';",
            r#""";"#,
            "$$$$;",
            "$1;",
            "/* x */ SELECT 1 -- y\n;",
        ] {
            assert_eq!(content_flags(sql), [true], "{sql:?}");
        }
    }

    #[test]
    fn whitespace_matches_postgres() {
        // Postgres 16+ lexes \v as whitespace but treats non-ASCII spaces as
        // identifier characters.
        assert_eq!(content_flags("\x0B\x0C;"), [false]);
        assert_eq!(content_flags("\u{a0};"), [true]);
    }

    #[test]
    fn content_resets_at_each_boundary() {
        assert_eq!(
            content_flags(";SELECT 1;; -- done"),
            [false, true, false, false]
        );
        assert_eq!(content_flags("-- x\n;SELECT 1"), [false, true]);
    }

    #[test]
    fn splits_after_multibyte_identifiers() {
        assert_eq!(
            statement_texts("SELECT café;SELECT 日本語"),
            ["SELECT café;", "SELECT 日本語"]
        );
    }

    #[test]
    fn waits_for_all_parentheses_to_close() {
        // Deliberately invalid SQL: the scanner finds boundaries, not syntax errors.
        assert_eq!(
            statement_texts("SELECT (1; (2; 3); 4);SELECT 5;"),
            ["SELECT (1; (2; 3); 4);", "SELECT 5;"]
        );
        assert_eq!(
            statement_texts("SELECT (1;SELECT 2;"),
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
            statement_texts("BEGIN;INSERT INTO t VALUES (1);COMMIT;"),
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
            statement_texts(&sql),
            [routine, "BEGIN;", "SELECT 3;", "COMMIT;", procedure]
        );
    }

    #[test]
    fn keeps_unfinished_routine_as_one_tail() {
        let sql = "CREATE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1; SELECT 2;";
        assert_eq!(statement_texts(sql), [sql]);
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
            statement_texts("SELECT '(';SELECT ')';SELECT 3;"),
            ["SELECT '(';", "SELECT ')';", "SELECT 3;"]
        );
    }

    #[test]
    fn ignores_block_keywords_inside_single_quotes() {
        let routine = "CREATE FUNCTION f() RETURNS text BEGIN ATOMIC SELECT 'end;'; END;";
        let sql = format!("{routine}SELECT 'begin';SELECT 3;");
        assert_eq!(
            statement_texts(&sql),
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
        assert_eq!(statement_texts(sql), [sql]);
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
        assert_eq!(statement_texts(&sql), [first, routine, "SELECT 3;"]);
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
        assert_eq!(statement_texts(sql), [sql]);
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
        assert_eq!(statement_texts(sql), [sql]);
    }

    #[test]
    fn line_comment_runs_to_end_of_line() {
        for statement in [
            "SELECT 1 -- a; b\n;",
            "SELECT 1 -- a; b\r;",
            "SELECT 1 -- a; b\r\n;",
            "-- first; line\nSELECT 1;",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn keeps_trailing_line_comment_as_its_own_tail() {
        assert_eq!(
            statement_texts("SELECT 1; -- done;"),
            ["SELECT 1;", " -- done;"]
        );
    }

    #[test]
    fn ignores_semicolons_inside_block_comments() {
        for statement in [
            "SELECT /* ; */ 1;",
            "SELECT /* /* ; */ ; */ 1;",
            "SELECT /*/ ; */ 1;",
            "SELECT /**/ 1;",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn ignores_quotes_inside_comments() {
        for statement in [
            "SELECT 1 -- it's\n;",
            "SELECT 1 /* it's */;",
            "SELECT 1 /* \" */;",
            "SELECT 1 /* e' */;",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn ignores_comment_markers_inside_quotes() {
        for statement in [
            "SELECT '--';",
            "SELECT '/*';",
            r#"SELECT 1 AS "--";"#,
            r"SELECT E'\'--';",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn lone_dash_and_slash_are_not_comments() {
        assert_not_split("SELECT 1 - 1 / 1 * 2;");
    }

    #[test]
    fn ignores_comment_markers_inside_other_comments() {
        assert_not_split("SELECT 1 -- /*\n;");
        assert_not_split("SELECT 1 /* -- */;");
    }

    #[test]
    fn comments_between_routine_keywords_are_ignored() {
        assert_not_split("CREATE /* x */ FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1; END;");
        assert_not_split(
            "CREATE -- x\nOR REPLACE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1; END;",
        );
    }

    #[test]
    fn ignores_block_keywords_inside_comments() {
        assert_not_split("CREATE FUNCTION f() RETURNS int -- begin\nRETURN 1;");
        assert_not_split("CREATE FUNCTION f() RETURNS int BEGIN ATOMIC SELECT 1 /* end; */; END;");
    }

    #[test]
    fn keeps_unterminated_block_comment_as_one_tail() {
        let sql = "SELECT 1 /* /* */ ; SELECT 2;";
        assert_eq!(statement_texts(sql), [sql]);
    }

    #[test]
    fn ignores_everything_inside_dollar_quotes() {
        for statement in [
            "SELECT $$a;b$$;",
            "SELECT $$ ( ' \" -- /* E' $$;",
            "SELECT $$$$;",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn dollar_quote_closes_only_on_matching_tag() {
        for statement in [
            "SELECT $tag$ ; $$ ; $tag$;",
            "SELECT $a$ ; $b$ ; $a$;",
            "SELECT $a$ ; $ba$ ; $a$;",
            "SELECT $A$ ; $a$ ; $A$;",
            "SELECT $t_1é$ ; $t_1é$;",
            "SELECT $a$ $5 ; a ; $a ; a$ ; $a$;",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn ignores_block_keywords_inside_dollar_quotes() {
        assert_not_split(
            "CREATE FUNCTION f() RETURNS int LANGUAGE plpgsql AS $$
BEGIN
    RETURN 1;
END;
$$;",
        );
        assert_not_split(
            "CREATE FUNCTION f() RETURNS int LANGUAGE plpgsql AS $body$
BEGIN
    RETURN (SELECT 1);
END;
$body$;",
        );
    }

    #[test]
    fn dollar_signs_that_do_not_open_a_quote() {
        for statement in [
            "SELECT $1;",
            "SELECT $1, $2;",
            "SELECT $1$ ;",
            "SELECT $tag ;",
            "SELECT a$b$ ;",
            "SELECT a$$ ;",
        ] {
            assert_not_split(statement);
        }
    }

    #[test]
    fn keeps_unterminated_dollar_quote_as_one_tail() {
        for sql in ["SELECT $$ ; SELECT 2;", "SELECT $a$ ; $b$ ; SELECT 2;"] {
            assert_eq!(statement_texts(sql), [sql], "{sql}");
        }
    }

    #[test]
    fn starts_are_byte_offsets() {
        let starts: Vec<usize> = split_statements("SELECT 1;\nSELECT 'é';SELECT 3")
            .iter()
            .map(|s| s.start)
            .collect();
        assert_eq!(starts, [0, 9, 22]);
    }

    #[test]
    fn statements_cover_the_input_in_order() {
        for sql in [
            "SELECT 1;SELECT 2;SELECT 3",
            "  SELECT 1;\n\tSELECT 2;\n",
            ";SELECT 'a;b';;",
            "SELECT café; -- done;\nSELECT $$日本;語$$;",
        ] {
            let mut expected_start = 0;
            for statement in split_statements(sql) {
                assert_eq!(statement.start, expected_start, "{sql}");
                expected_start += statement.text.len();
            }
            assert_eq!(expected_start, sql.len(), "{sql}");
        }
    }
}
