// Rough port of parts of @postgres/postgres/blob/master/src/fe_utils/psqlscan.l
// for splitting a SQL string on statement boundaries.

// Identifier keywords that impact scanning behavior
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

#[derive(Default)]
struct ScanState {
    paren_depth: i32,
    begin_depth: i32,
    init_idents: [Keyword; 4],
    init_idents_count: usize,
}

impl ScanState {
    // Port of psqlscan_track_identifier
    fn track_identifier(&mut self, identifier: &str) {
        if self.paren_depth != 0 {
            return;
        }

        if self.init_idents_count == 0 {
            self.init_idents.fill(Keyword::Other);
        }
        self.record_initial_keyword(identifier);

        if self.is_create_routine() {
            if identifier.eq_ignore_ascii_case("begin") {
                self.begin_depth += 1;
            } else if identifier.eq_ignore_ascii_case("case") && self.begin_depth > 0 {
                self.begin_depth += 1;
            } else if identifier.eq_ignore_ascii_case("end") && self.begin_depth > 0 {
                self.begin_depth -= 1;
            }
        }
    }

    fn record_initial_keyword(&mut self, identifier: &str) {
        if self.init_idents_count < self.init_idents.len() {
            self.init_idents[self.init_idents_count] = Keyword::from_identifier(identifier);
            self.init_idents_count += 1;
        }
    }

    // Does the current input match CREATE [OR REPLACE] {FUNCTION|PROCEDURE}?
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

    // Start of an identifier: [A-Za-z\200-\377_]
    fn ident_start(c: char) -> bool {
        c.is_ascii_alphabetic() || !c.is_ascii() || c == '_'
    }

    // Continuation of an identifier: [A-Za-z\200-\377_0-9\$]
    fn ident_cont(c: char) -> bool {
        ident_start(c) || c.is_ascii_digit() || c == '$'
    }

    while let Some((i, c)) = chars.next() {
        match c {
            '(' => {
                state.paren_depth += 1;
            }
            ')' => {
                if state.paren_depth > 0 {
                    state.paren_depth -= 1;
                }
            }
            ';' if state.paren_depth == 0 && state.begin_depth == 0 => {
                statements.push(&sql[statement_start..i + 1]);
                statement_start = i + 1;
                state.init_idents_count = 0;
            }
            c if ident_start(c) => {
                while let Some(&(_, c_next)) = chars.peek() {
                    if !ident_cont(c_next) {
                        break;
                    }
                    chars.next();
                }

                let end = match chars.peek() {
                    Some(&(i_next, _)) => i_next,
                    None => sql.len(),
                };

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
