//! Turning what someone typed into something FTS5 will accept.
//!
//! The match syntax has operators, and a person searching for dialogue has no
//! reason to know that. Someone typing `AND` means the word, and someone typing
//! a bracket means the bracket. So every term is quoted on the way in, which
//! makes the whole query literal, and the only syntax left is the one thing
//! people do expect: double quotes around a phrase.
//!
//! A query that reduces to nothing is not an error either. It is the state the
//! search field is in for the first keystroke of every search anyone makes.

/// One piece of a query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Term<'a> {
    text: &'a str,
    /// Whether the person wrote it inside quotes and closed them, which is what
    /// stops the last term from being treated as half typed.
    phrase: bool,
}

/// Builds a match expression, or `None` if there is nothing to search for.
///
/// The last term is matched as a prefix so that results appear while someone is
/// still typing the word. A closed phrase is taken as finished and matched
/// whole.
pub(crate) fn build(input: &str) -> Option<String> {
    let mut terms: Vec<Term<'_>> = Vec::new();
    let mut rest = input.trim_start();

    while !rest.is_empty() {
        let (term, remainder) = if let Some(quoted) = rest.strip_prefix('"') {
            match quoted.find('"') {
                Some(end) => (
                    Term {
                        text: &quoted[..end],
                        phrase: true,
                    },
                    &quoted[end + 1..],
                ),
                // A quote nobody has closed yet, which is what a phrase looks
                // like half way through being typed.
                None => (
                    Term {
                        text: quoted,
                        phrase: false,
                    },
                    "",
                ),
            }
        } else {
            let end = rest
                .find(|character: char| character.is_whitespace() || character == '"')
                .unwrap_or(rest.len());
            (
                Term {
                    text: &rest[..end],
                    phrase: false,
                },
                &rest[end..],
            )
        };

        // Punctuation on its own carries no term for the tokeniser to match, and
        // quoting it would produce an empty phrase, which is a syntax error.
        if term.text.chars().any(char::is_alphanumeric) {
            terms.push(term);
        }
        rest = remainder.trim_start();
    }

    let last = terms.len().checked_sub(1)?;
    let mut expression = String::with_capacity(input.len() + terms.len() * 4);
    for (at, term) in terms.iter().enumerate() {
        if at > 0 {
            expression.push_str(" AND ");
        }
        expression.push('"');
        expression.push_str(&term.text.replace('"', "\"\""));
        expression.push('"');
        if at == last && !term.phrase {
            expression.push('*');
        }
    }
    Some(expression)
}

#[cfg(test)]
mod tests {
    use super::build;

    #[test]
    fn a_single_word_matches_as_a_prefix() {
        assert_eq!(build("hello").as_deref(), Some("\"hello\"*"));
    }

    #[test]
    fn words_all_have_to_appear() {
        assert_eq!(
            build("hello there").as_deref(),
            Some("\"hello\" AND \"there\"*")
        );
    }

    #[test]
    fn a_closed_phrase_is_matched_whole() {
        assert_eq!(
            build("\"good morning\"").as_deref(),
            Some("\"good morning\"")
        );
        assert_eq!(
            build("\"good morning\" ins").as_deref(),
            Some("\"good morning\" AND \"ins\"*")
        );
    }

    #[test]
    fn a_phrase_still_being_typed_is_matched_as_a_prefix() {
        assert_eq!(build("\"good morn").as_deref(), Some("\"good morn\"*"));
    }

    #[test]
    fn operators_are_read_as_words() {
        // Someone searching for this wants the line "war and peace", not a
        // conjunction of two terms and certainly not a syntax error.
        assert_eq!(
            build("war AND peace").as_deref(),
            Some("\"war\" AND \"AND\" AND \"peace\"*")
        );
        assert_eq!(
            build("NEAR(a b)").as_deref(),
            Some("\"NEAR(a\" AND \"b)\"*")
        );
    }

    #[test]
    fn punctuation_cannot_break_the_expression() {
        for input in [
            "-hello",
            "^hello",
            "hello*",
            "(hello)",
            "hello: \"world",
            "50%",
            "don't",
        ] {
            let expression = build(input).unwrap_or_default();
            assert!(!expression.is_empty(), "{input} produced nothing");
            // Every quote in the expression is either one we added or one we
            // doubled, so they always balance.
            assert_eq!(expression.matches('"').count() % 2, 0, "{input}");
        }
    }

    #[test]
    fn a_query_with_nothing_in_it_is_not_a_query() {
        for input in ["", "   ", "\"\"", "()", "- -", "***"] {
            assert_eq!(build(input), None, "{input} should produce nothing");
        }
    }

    #[test]
    fn a_quote_left_open_after_a_phrase_is_no_trouble() {
        assert_eq!(
            build("\"she said\" \"yes").as_deref(),
            Some("\"she said\" AND \"yes\"*")
        );
    }
}
