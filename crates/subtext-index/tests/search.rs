//! Finding a line of dialogue.

#![allow(clippy::unwrap_used)]

mod common;

use subtext_index::{MATCH_END, MATCH_START, SearchOptions};

use crate::common::Library;

/// A library of three films with a handful of lines each.
fn library() -> Library {
    let library = Library::new();
    let folder = library.watch();

    library.add_film_with_dialogue(
        folder,
        "Heat",
        &[
            "Don't let yourself get attached to anything",
            "A guy told me one time, don't let yourself get attached",
            "The action is the juice",
        ],
    );
    library.add_film_with_dialogue(
        folder,
        "Ronin",
        &[
            "Whenever there is any doubt, there is no doubt",
            "I never walk into a place I don't know how to walk out of",
            "The action is elsewhere",
        ],
    );
    library.add_film_with_dialogue(
        folder,
        "Casablanca",
        &[
            "We'll always have Paris",
            "Round up the usual suspects",
            "Here's looking at you, kid",
            "Of all the gin joints in all the towns in all the world",
        ],
    );

    library
}

#[test]
fn finds_a_line_and_says_where_it_is() {
    let library = library();
    let results = library
        .database
        .search()
        .find("Paris", &SearchOptions::default())
        .unwrap();

    assert_eq!(results.films.len(), 1);
    let film = &results.films[0];
    assert_eq!(film.title, "Casablanca");
    assert_eq!(film.year, Some(1_999));
    assert_eq!(film.hits.len(), 1);
    assert_eq!(film.hits[0].text, "We'll always have Paris");
    assert_eq!(film.hits[0].start.millis(), 0);
    assert!(!results.truncated);
}

#[test]
fn groups_by_film_and_marks_what_matched() {
    let library = library();
    let results = library
        .database
        .search()
        .find("action", &SearchOptions::default())
        .unwrap();

    assert_eq!(results.films.len(), 2);
    assert_eq!(results.shown(), 2);

    let snippet = &results.films[0].hits[0].snippet;
    assert!(
        snippet.contains(&format!("{MATCH_START}action{MATCH_END}")),
        "the match should be marked: {snippet}"
    );
}

#[test]
fn a_film_gets_a_share_of_the_results_rather_than_all_of_them() {
    let library = library();
    let options = SearchOptions {
        per_film: 1,
        ..SearchOptions::default()
    };
    let results = library
        .database
        .search()
        .find("attached", &options)
        .unwrap();

    assert_eq!(results.films.len(), 1);
    assert_eq!(results.films[0].hits.len(), 1);
    assert_eq!(
        results.films[0].withheld, 1,
        "the second line in the same film should be counted, not dropped silently"
    );
}

#[test]
fn a_search_can_be_kept_to_the_film_being_watched() {
    let library = library();
    let everywhere = library
        .database
        .search()
        .find("action", &SearchOptions::default())
        .unwrap();
    let film_id = everywhere.films[0].film_id;

    let scoped = library
        .database
        .search()
        .find(
            "action",
            &SearchOptions {
                film: Some(film_id),
                ..SearchOptions::default()
            },
        )
        .unwrap();

    assert_eq!(scoped.films.len(), 1);
    assert_eq!(scoped.films[0].film_id, film_id);
}

#[test]
fn matches_a_word_as_it_is_being_typed() {
    let library = library();
    let options = SearchOptions::default();

    for typed in ["s", "sus", "suspect"] {
        let results = library.database.search().find(typed, &options).unwrap();
        assert!(
            results
                .films
                .iter()
                .flat_map(|film| &film.hits)
                .any(|hit| hit.text.contains("usual suspects")),
            "{typed} should already be finding the line"
        );
    }
}

#[test]
fn a_phrase_in_quotes_matches_in_that_order() {
    let library = library();
    let options = SearchOptions::default();

    assert_eq!(
        library
            .database
            .search()
            .find("\"the action is\"", &options)
            .unwrap()
            .shown(),
        2
    );
    assert!(
        library
            .database
            .search()
            .find("\"action the is\"", &options)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn words_are_matched_by_their_stem() {
    let library = library();
    // "walk" in the file, "walking" in the search field.
    let results = library
        .database
        .search()
        .find("walking", &SearchOptions::default())
        .unwrap();

    assert_eq!(results.films.len(), 1);
    assert!(results.films[0].hits[0].text.contains("walk into a place"));
}

#[test]
fn punctuation_in_the_query_finds_lines_rather_than_failing() {
    let library = library();
    let options = SearchOptions::default();

    // Every one of these is FTS5 syntax of some kind, and none of them is
    // anything but text to the person typing it.
    for query in [
        "don't",
        "you, kid",
        "(Paris)",
        "action -juice",
        "NEAR(action juice)",
        "\"unclosed",
        "action AND juice",
        "*",
        "^^^",
    ] {
        let results = library.database.search().find(query, &options);
        assert!(results.is_ok(), "{query} should not be an error");
    }

    // An empty search is the state the field is in before anyone types.
    assert!(
        library
            .database
            .search()
            .find("", &options)
            .unwrap()
            .is_empty()
    );
    assert!(
        library
            .database
            .search()
            .find("   ", &options)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn says_when_it_stopped_at_the_limit() {
    let library = library();
    let options = SearchOptions {
        limit: 1,
        ..SearchOptions::default()
    };
    let results = library.database.search().find("the", &options).unwrap();

    assert_eq!(results.shown(), 1);
    assert!(results.truncated);
}

#[test]
fn a_search_matching_half_the_library_gives_up_ranking_and_says_so() {
    let library = library();

    let ranked = library
        .database
        .search()
        .find("action", &SearchOptions::default())
        .unwrap();
    assert!(ranked.ranked);

    // "the" is what someone types on the way to "the action", and matching
    // everything is exactly when ranking costs the most and is worth the least.
    let beyond = library
        .database
        .search()
        .find(
            "the",
            &SearchOptions {
                ranking_budget: 2,
                ..SearchOptions::default()
            },
        )
        .unwrap();

    assert!(!beyond.ranked, "ranking should have been given up on");
    assert!(
        beyond.shown() > 2,
        "the lines still come back, in library order"
    );
}

#[test]
fn a_search_for_something_nobody_says_finds_nothing() {
    let library = library();
    let results = library
        .database
        .search()
        .find("helicopter", &SearchOptions::default())
        .unwrap();

    assert!(results.is_empty());
    assert!(!results.truncated);
}
