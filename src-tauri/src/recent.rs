//! The searches somebody has made before.
//!
//! Kept in the preference table rather than in a table of their own. It is one
//! short list belonging to the application rather than to any film, it is read
//! whole and written whole, and a table with a row per search would be a
//! migration and an index for something that never grows past eight rows.
//!
//! A search is remembered when it led somewhere, not on every keystroke.
//! Otherwise the list would fill with every prefix of the last thing typed:
//! `p`, `pa`, `par`, `pari`, `paris`.

use subtext_index::Database;

use crate::dto::Failure;

/// The preference the list lives under.
const KEY: &str = "search.recent";

/// How many searches are kept.
///
/// Enough to come back to something from earlier in the evening, few enough
/// that the palette can show the lot before anybody has typed anything.
const KEPT: usize = 8;

/// What has been searched for, most recent first.
///
/// A preference holding something other than a list of searches reads as no
/// searches. The alternative is a palette that will not open because of a value
/// nobody can see and nothing can fix.
pub(crate) fn list(database: &Database) -> Result<Vec<String>, Failure> {
    let stored = database.preferences().get(KEY).map_err(Failure::of)?;
    Ok(stored
        .and_then(|json| serde_json::from_str::<Vec<String>>(&json).ok())
        .unwrap_or_default())
}

/// Puts a search at the top of the list and hands back the list it made.
pub(crate) fn remember(database: &Database, query: &str) -> Result<Vec<String>, Failure> {
    let searches = updated(list(database)?, query);
    let json = serde_json::to_string(&searches).map_err(Failure::of)?;
    database
        .preferences()
        .set(KEY, &json)
        .map_err(Failure::of)?;

    Ok(searches)
}

/// Forgets every search, for somebody who would rather it did not know.
pub(crate) fn forget(database: &Database) -> Result<(), Failure> {
    database.preferences().remove(KEY).map_err(Failure::of)?;
    Ok(())
}

/// The list one search leaves behind it.
///
/// Searching for something already in the list moves it to the top rather than
/// putting it there twice, and the case it was typed in the second time is the
/// one kept: it is the more recent decision, and a list that answered a
/// deliberate `Paris` with the `paris` from last week would read as a typo.
fn updated(existing: Vec<String>, query: &str) -> Vec<String> {
    let query = query.trim();
    if query.is_empty() {
        return existing;
    }

    let mut searches = Vec::with_capacity(existing.len() + 1);
    searches.push(query.to_owned());
    searches.extend(
        existing
            .into_iter()
            .filter(|kept| !kept.eq_ignore_ascii_case(query)),
    );
    searches.truncate(KEPT);
    searches
}

#[cfg(test)]
mod tests {
    use super::{KEPT, updated};

    fn searches(list: &[&str]) -> Vec<String> {
        list.iter().map(|search| (*search).to_owned()).collect()
    }

    #[test]
    fn the_latest_search_comes_first() {
        let list = updated(searches(&["paris", "juice"]), "helicopter");
        assert_eq!(list, searches(&["helicopter", "paris", "juice"]));
    }

    #[test]
    fn searching_for_the_same_thing_again_moves_it_rather_than_repeating_it() {
        let list = updated(searches(&["paris", "juice", "kid"]), "juice");
        assert_eq!(list, searches(&["juice", "paris", "kid"]));
    }

    #[test]
    fn the_case_it_was_typed_in_this_time_is_the_one_kept() {
        let list = updated(searches(&["paris"]), "Paris");
        assert_eq!(list, searches(&["Paris"]));
    }

    #[test]
    fn only_so_many_are_kept() {
        let mut list = searches(&[]);
        for search in ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"] {
            list = updated(list, search);
        }

        assert_eq!(list.len(), KEPT);
        assert_eq!(list.first().map(String::as_str), Some("j"));
        assert!(!list.iter().any(|kept| kept == "a"));
    }

    #[test]
    fn a_search_is_kept_without_the_space_somebody_left_on_the_end() {
        let list = updated(searches(&[]), "  the action  ");
        assert_eq!(list, searches(&["the action"]));
    }

    #[test]
    fn a_search_with_nothing_in_it_is_not_a_search() {
        let before = searches(&["paris"]);
        assert_eq!(updated(before.clone(), ""), before);
        assert_eq!(updated(before.clone(), "   "), before);
    }
}
