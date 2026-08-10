//! Searching every line of dialogue in the library.

use std::collections::HashMap;

use rusqlite::ToSql;
use subtext_core::Timestamp;

use crate::database::Database;
use crate::error::Result;
use crate::repository::{count_to_sql, from_sql_count, query};

/// What a snippet puts in front of a match.
///
/// Control characters rather than markup: the front end splits the snippet on
/// them and renders the pieces, so a line of dialogue containing a less-than
/// sign cannot become an element on the way to the screen.
pub const MATCH_START: &str = "\u{2}";
/// What a snippet puts after a match. See [`MATCH_START`].
pub const MATCH_END: &str = "\u{3}";

/// How much of the line either side of a match a snippet keeps.
const SNIPPET_TOKENS: i64 = 12;

#[derive(Clone, Copy, Debug)]
pub struct SearchOptions {
    /// Only this film, which is what Ctrl+K does while something is playing.
    pub film: Option<i64>,
    /// Most lines to return across the whole library.
    pub limit: usize,
    /// Most lines to show under any one film, so that a film that says the word
    /// forty times does not fill the palette.
    pub per_film: usize,
    pub highlight: (&'static str, &'static str),
    /// How many matching lines are worth ranking before ranking stops being
    /// worth the wait. See [`SearchResults::ranked`].
    pub ranking_budget: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            film: None,
            limit: 100,
            per_film: 5,
            highlight: (MATCH_START, MATCH_END),
            // Ranking scores every matching line, and finding those lines costs
            // more again for a phrase, which has to walk the whole list of the
            // commonest word in it. Two thousand keeps the worst query measured
            // inside ten milliseconds, well within the fifty the palette has
            // between keystrokes, and no search returning more lines than that
            // has a best line worth waiting for.
            ranking_budget: 2_000,
        }
    }
}

/// One line of dialogue that matched.
#[derive(Clone, Debug)]
pub struct CueHit {
    pub cue_id: i64,
    pub track_id: i64,
    pub start: Timestamp,
    pub end: Timestamp,
    /// The line as it is spoken.
    pub text: String,
    /// The line around the match, with the matched words marked.
    pub snippet: String,
}

/// The matches within one film, best first.
#[derive(Clone, Debug)]
pub struct FilmHits {
    pub film_id: i64,
    pub title: String,
    pub year: Option<u16>,
    pub hits: Vec<CueHit>,
    /// Matches in this film that were not returned, because the film reached
    /// its share of the results.
    pub withheld: usize,
}

#[derive(Clone, Debug, Default)]
pub struct SearchResults {
    /// Films in the order their best line ranked.
    pub films: Vec<FilmHits>,
    /// Whether the search stopped at the limit rather than running out of
    /// matches, so the front end can say "the first hundred" honestly.
    pub truncated: bool,
    /// Whether these are the best matches or simply the first ones.
    ///
    /// Ranking reads a score for every matching line, which is quick for the
    /// searches people mean and slow for the ones they pass through on the way:
    /// typing "the" on the way to "the money" matches half the library. Past
    /// the budget the lines come back in library order instead, which is worth
    /// far more than a perfect ordering nobody waited to see.
    pub ranked: bool,
}

impl SearchResults {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.films.is_empty()
    }

    /// How many lines are being shown.
    #[must_use]
    pub fn shown(&self) -> usize {
        self.films.iter().map(|film| film.hits.len()).sum()
    }
}

#[derive(Debug)]
pub struct Search<'a> {
    database: &'a Database,
}

impl<'a> Search<'a> {
    pub(crate) fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Finds lines of dialogue, ranked by BM25 and grouped by film.
    ///
    /// Ranking happens in SQLite over the whole index, and the grouping happens
    /// here over the rows that came back. Doing it the other way round, a query
    /// per film, would be a query per film.
    pub fn find(&self, text: &str, options: &SearchOptions) -> Result<SearchResults> {
        let Some(expression) = query::build(text) else {
            return Ok(SearchResults {
                ranked: true,
                ..SearchResults::default()
            });
        };

        self.database.with(|connection| {
            // Searching within one film means searching a few thousand cues out
            // of a million, and the identifiers of a film's cues sit together,
            // since they were written together. Handing the index that span
            // turns the scan into a seek. The join below still decides what
            // belongs to the film, so a span that takes in a neighbour's cues
            // costs a little work and changes no answer.
            let span = match options.film {
                Some(film) => match cue_span(connection, film)? {
                    Some(span) => Some(span),
                    None => return Ok(SearchResults::default()),
                },
                None => None,
            };

            let ranked =
                within_ranking_budget(connection, &expression, options.ranking_budget, span)?;
            let filter = if span.is_some() {
                "AND f.id = ?6 AND cue_search.rowid BETWEEN ?7 AND ?8"
            } else {
                ""
            };
            // Without an order, matches come back by cue identifier, which is
            // the order they were indexed in: film by film, and within a film
            // from the opening line onwards.
            let order = if ranked {
                "ORDER BY bm25(cue_search)"
            } else {
                ""
            };
            let sql = format!(
                "SELECT c.id, c.track_id, c.start_ms, c.end_ms, c.text,
                        snippet(cue_search, 0, ?2, ?3, '…', ?4),
                        f.id, f.title, f.year
                 FROM cue_search
                 JOIN cue AS c ON c.id = cue_search.rowid
                 JOIN subtitle_track AS t ON t.id = c.track_id
                 JOIN film AS f ON f.id = t.film_id
                 WHERE cue_search MATCH ?1 {filter}
                 {order}
                 LIMIT ?5"
            );

            let limit = count_to_sql(options.limit);
            let mut parameters: Vec<&dyn ToSql> = vec![
                &expression,
                &options.highlight.0,
                &options.highlight.1,
                &SNIPPET_TOKENS,
                &limit,
            ];
            if let (Some(film), Some((first, last))) = (options.film.as_ref(), span.as_ref()) {
                parameters.push(film);
                parameters.push(first);
                parameters.push(last);
            }

            let mut statement = connection.prepare(&sql)?;
            let mut rows = statement.query(parameters.as_slice())?;

            let mut films: Vec<FilmHits> = Vec::new();
            let mut position: HashMap<i64, usize> = HashMap::new();
            let mut returned = 0_usize;

            while let Some(row) = rows.next()? {
                returned += 1;
                let film_id: i64 = row.get(6)?;
                let at = if let Some(at) = position.get(&film_id) {
                    *at
                } else {
                    films.push(FilmHits {
                        film_id,
                        title: row.get(7)?,
                        year: row
                            .get::<_, Option<i64>>(8)?
                            .and_then(|year| u16::try_from(year).ok()),
                        hits: Vec::new(),
                        withheld: 0,
                    });
                    position.insert(film_id, films.len() - 1);
                    films.len() - 1
                };

                let Some(film) = films.get_mut(at) else {
                    continue;
                };
                if film.hits.len() >= options.per_film {
                    film.withheld += 1;
                    continue;
                }
                film.hits.push(CueHit {
                    cue_id: row.get(0)?,
                    track_id: row.get(1)?,
                    start: Timestamp::from_millis(row.get(2)?),
                    end: Timestamp::from_millis(row.get(3)?),
                    text: row.get(4)?,
                    snippet: row.get(5)?,
                });
            }

            Ok(SearchResults {
                films,
                truncated: returned >= options.limit,
                ranked,
            })
        })
    }
}

/// Whether there are few enough matches for ranking them to be quick.
///
/// Reading the identifiers of the first few thousand matches costs a millisecond
/// or two, and reading one more than the budget is enough to know. Scoring them
/// all, which is what `ORDER BY rank` does, costs a few microseconds each, so
/// this is the cheap question that stops the expensive one being asked.
fn within_ranking_budget(
    connection: &rusqlite::Connection,
    expression: &str,
    budget: usize,
    span: Option<(i64, i64)>,
) -> Result<bool> {
    let ceiling = count_to_sql(budget) + 1;
    let seen: i64 = match span {
        Some((first, last)) => connection.query_row(
            "SELECT count(*) FROM (
                 SELECT rowid FROM cue_search
                 WHERE cue_search MATCH ?1 AND rowid BETWEEN ?3 AND ?4
                 LIMIT ?2
             )",
            rusqlite::params![expression, ceiling, first, last],
            |row| row.get(0),
        )?,
        None => connection.query_row(
            "SELECT count(*) FROM (
                 SELECT rowid FROM cue_search WHERE cue_search MATCH ?1 LIMIT ?2
             )",
            rusqlite::params![expression, ceiling],
            |row| row.get(0),
        )?,
    };
    Ok(from_sql_count(seen) <= budget)
}

/// The first and last cue identifiers belonging to a film.
fn cue_span(connection: &rusqlite::Connection, film: i64) -> Result<Option<(i64, i64)>> {
    let span = connection.query_row(
        "SELECT min(c.id), max(c.id) FROM cue AS c
         JOIN subtitle_track AS t ON t.id = c.track_id
         WHERE t.film_id = ?1",
        [film],
        |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
    )?;
    Ok(match span {
        (Some(first), Some(last)) => Some((first, last)),
        // A film with no cues yet, which has nothing to search.
        _ => None,
    })
}
