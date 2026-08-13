//! Working out which subtitle file belongs to which film.
//!
//! The only evidence available is the names, since nothing inside an SRT file
//! says what it is a subtitle for. Names are therefore reduced to a comparable
//! form and matched in two passes: first on the names being the same, then on
//! one name containing the other, which is what catches a film whose file
//! carries a release group and a subtitle whose file does not. The second pass
//! is a preference: see [`Matching`].
//!
//! Where the evidence is not good enough the pairing is left undone rather
//! than guessed at. An unpaired subtitle is a row in the import sheet with an
//! attach button next to it, which is a far smaller annoyance than a film
//! quietly showing someone else's dialogue.

mod language;
mod name;

pub use self::language::{SubtitleLabel, language_code};
pub use self::name::ParsedName;

/// How sure the pairing is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchKind {
    /// The two names reduced to exactly the same thing.
    Exact,
    /// One name is the beginning of the other, word for word.
    Approximate,
}

/// How much evidence a pairing needs before it is made.
///
/// A preference rather than a constant, because the right answer depends on the
/// folder. A library named by hand pairs exactly and anything approximate in it
/// is a mistake waiting to happen; a library of downloads is full of films
/// carrying a release group and subtitles that do not, and refusing those means
/// attaching half of them by hand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Matching {
    /// One name beginning the other, word for word, is enough.
    #[default]
    Relaxed,
    /// Only names that reduce to exactly the same thing.
    Exact,
}

/// A subtitle file and the film it was paired with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubtitleMatch {
    /// Index into the subtitles given to [`pair`].
    pub subtitle: usize,
    /// Index into the films given to [`pair`].
    pub film: usize,
    pub kind: MatchKind,
}

/// What the pairing found, ready for the import sheet to show.
#[derive(Clone, Debug)]
pub struct PairingReport {
    /// The films as they were read, in the order they were given.
    pub films: Vec<ParsedName>,
    /// The subtitles as they were read, in the order they were given.
    pub subtitles: Vec<ParsedName>,
    pub matches: Vec<SubtitleMatch>,
    /// Subtitles with no film, or with more than one equally good film.
    pub unmatched_subtitles: Vec<usize>,
    pub films_without_subtitles: Vec<usize>,
}

impl PairingReport {
    /// The subtitles paired with one film. A film may have several, one per
    /// language.
    pub fn subtitles_for(&self, film: usize) -> impl Iterator<Item = &SubtitleMatch> {
        self.matches.iter().filter(move |found| found.film == film)
    }
}

/// Pairs subtitle files with films by name, taking whatever evidence there is.
///
/// Both slices hold file names, not paths. A film may end up with several
/// subtitles; a subtitle belongs to at most one film.
#[must_use]
pub fn pair<F: AsRef<str>, S: AsRef<str>>(films: &[F], subtitles: &[S]) -> PairingReport {
    pair_with(films, subtitles, Matching::Relaxed)
}

/// The same, holding out for as much evidence as was asked for.
#[must_use]
pub fn pair_with<F: AsRef<str>, S: AsRef<str>>(
    films: &[F],
    subtitles: &[S],
    matching: Matching,
) -> PairingReport {
    let films: Vec<ParsedName> = films
        .iter()
        .map(|name| ParsedName::from_file_name(name.as_ref()))
        .collect();
    let subtitles: Vec<ParsedName> = subtitles
        .iter()
        .map(|name| ParsedName::from_file_name(name.as_ref()))
        .collect();

    let mut matches = Vec::new();
    let mut unmatched_subtitles = Vec::new();

    for (at, subtitle) in subtitles.iter().enumerate() {
        match find_film(&films, subtitle, matching) {
            Some((film, kind)) => matches.push(SubtitleMatch {
                subtitle: at,
                film,
                kind,
            }),
            None => unmatched_subtitles.push(at),
        }
    }

    let films_without_subtitles = (0..films.len())
        .filter(|film| !matches.iter().any(|found| found.film == *film))
        .collect();

    PairingReport {
        films,
        subtitles,
        matches,
        unmatched_subtitles,
        films_without_subtitles,
    }
}

fn find_film(
    films: &[ParsedName],
    subtitle: &ParsedName,
    matching: Matching,
) -> Option<(usize, MatchKind)> {
    if subtitle.key.is_empty() {
        return None;
    }

    let mut exact = films
        .iter()
        .enumerate()
        .filter(|(_, film)| film.key == subtitle.key && film.year_agrees_with(subtitle));
    if let Some((at, _)) = exact.next() {
        // Two films reducing to the same name is a question only the person
        // whose files they are can answer.
        return exact.next().is_none().then_some((at, MatchKind::Exact));
    }

    if matching == Matching::Exact {
        return None;
    }

    let mut best: Option<(usize, usize)> = None;
    let mut tied = false;
    for (at, film) in films.iter().enumerate() {
        if !film.year_agrees_with(subtitle) {
            continue;
        }
        let Some(shared) = shared_prefix(&film.key_words, &subtitle.key_words) else {
            continue;
        };
        match best {
            Some((_, most)) if shared < most => {}
            Some((_, most)) if shared == most => tied = true,
            _ => {
                best = Some((at, shared));
                tied = false;
            }
        }
    }

    match best {
        Some((at, _)) if !tied => Some((at, MatchKind::Approximate)),
        _ => None,
    }
}

/// How many words the two names share, when one is the beginning of the other.
///
/// Whole words rather than letters, so that Ronin does not begin The Ronin
/// Affair. A single short word is refused outright: Up would otherwise be the
/// beginning of Upgrade, and a one word title is exactly where a wrong pairing
/// is most likely.
fn shared_prefix(film: &[String], subtitle: &[String]) -> Option<usize> {
    let shared = film.len().min(subtitle.len());
    if shared == 0 || film[..shared] != subtitle[..shared] {
        return None;
    }
    if shared == 1 && film[0].len() < 4 {
        return None;
    }
    Some(shared)
}

#[cfg(test)]
mod tests {
    use super::{MatchKind, Matching, pair, pair_with};

    #[test]
    fn pairs_names_that_reduce_to_the_same_thing() {
        let report = pair(
            &["The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv"],
            &["The Matrix (1999).srt"],
        );

        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].film, 0);
        assert_eq!(report.matches[0].kind, MatchKind::Exact);
        assert!(report.unmatched_subtitles.is_empty());
        assert!(report.films_without_subtitles.is_empty());
    }

    #[test]
    fn pairs_a_shorter_name_with_a_longer_one() {
        let report = pair(&["Arrival.2016.1080p.mkv"], &["Arrival Original.srt"]);

        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].kind, MatchKind::Approximate);
    }

    #[test]
    fn holding_out_for_an_exact_name_refuses_a_shorter_one() {
        let films = ["Arrival.2016.1080p.mkv"];
        let subtitles = ["Arrival Original.srt"];

        let report = pair_with(&films, &subtitles, Matching::Exact);
        assert!(report.matches.is_empty());
        assert_eq!(report.unmatched_subtitles, [0]);
        assert_eq!(report.films_without_subtitles, [0]);

        // The same pair, taking whatever evidence there is.
        assert_eq!(
            pair_with(&films, &subtitles, Matching::Relaxed)
                .matches
                .len(),
            1
        );
    }

    #[test]
    fn holding_out_for_an_exact_name_still_pairs_the_names_that_agree() {
        let report = pair_with(
            &["The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv"],
            &["The Matrix (1999).srt"],
            Matching::Exact,
        );

        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].kind, MatchKind::Exact);
    }

    #[test]
    fn taking_whatever_evidence_there_is_is_what_pairing_does_by_default() {
        let films = ["Arrival.2016.1080p.mkv"];
        let subtitles = ["Arrival Original.srt"];

        assert_eq!(
            pair(&films, &subtitles).matches,
            pair_with(&films, &subtitles, Matching::default()).matches
        );
    }

    #[test]
    fn a_film_may_have_a_subtitle_in_every_language() {
        let report = pair(
            &["Parasite.2019.1080p.mkv"],
            &[
                "Parasite.2019.en.srt",
                "Parasite.2019.ko.srt",
                "Parasite.2019.fr.forced.srt",
            ],
        );

        assert_eq!(report.matches.len(), 3);
        assert_eq!(report.subtitles_for(0).count(), 3);
        assert_eq!(report.subtitles[1].label.language, Some("ko"));
        assert!(report.subtitles[2].label.forced);
    }

    #[test]
    fn the_wrong_film_in_the_same_folder_is_left_alone() {
        let report = pair(
            &["The.Matrix.1999.mkv", "The.Matrix.Reloaded.2003.mkv"],
            &["The.Matrix.Reloaded.2003.srt"],
        );

        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.matches[0].film, 1);
        assert_eq!(report.matches[0].kind, MatchKind::Exact);
        assert_eq!(report.films_without_subtitles, [0]);
    }

    #[test]
    fn a_year_that_disagrees_blocks_a_pairing() {
        let report = pair(&["Dune.2021.1080p.mkv"], &["Dune.1984.srt"]);

        assert!(report.matches.is_empty());
        assert_eq!(report.unmatched_subtitles, [0]);
        assert_eq!(report.films_without_subtitles, [0]);
    }

    #[test]
    fn a_choice_between_two_equally_good_films_is_left_to_a_person() {
        // Same title, no year on either, in one folder. Nothing in the names
        // can settle it.
        let report = pair(&["Solaris.mkv", "Solaris.mp4"], &["Solaris.srt"]);
        assert!(report.matches.is_empty());
        assert_eq!(report.unmatched_subtitles, [0]);

        // The same again for the approximate pass.
        let ambiguous = pair(
            &["Solaris Part One.mkv", "Solaris Part Two.mkv"],
            &["Solaris.srt"],
        );
        assert!(ambiguous.matches.is_empty());
    }

    #[test]
    fn one_name_beginning_another_is_not_enough_on_its_own() {
        // Words rather than letters, so Up does not begin Upgrade.
        assert!(pair(&["Upgrade.2018.mkv"], &["Up.srt"]).matches.is_empty());

        // And one short word is too little to go on, even where it does match.
        assert!(
            pair(&["Up.The.Creek.1984.mkv"], &["Up.srt"])
                .matches
                .is_empty()
        );

        // The same short name, paired exactly, is no trouble at all.
        let exact = pair(&["Up.2009.1080p.mkv"], &["Up.2009.srt"]);
        assert_eq!(exact.matches.len(), 1);
        assert_eq!(exact.matches[0].kind, MatchKind::Exact);
    }

    #[test]
    fn subtitles_with_no_film_are_reported_rather_than_dropped() {
        let report = pair(&["Heat.1995.mkv"], &["Heat.1995.srt", "Ronin.1998.srt"]);

        assert_eq!(report.matches.len(), 1);
        assert_eq!(report.unmatched_subtitles, [1]);
    }

    #[test]
    fn nothing_pairs_with_nothing() {
        let report = pair::<&str, &str>(&[], &[]);
        assert!(report.matches.is_empty());
        assert!(report.unmatched_subtitles.is_empty());
        assert!(report.films_without_subtitles.is_empty());

        let no_films = pair(&[] as &[&str], &["Heat.1995.srt"]);
        assert_eq!(no_films.unmatched_subtitles, [0]);
    }

    #[test]
    fn a_name_with_nothing_in_it_pairs_with_nothing() {
        let report = pair(&["Heat.1995.mkv"], &[".srt", "----.srt"]);
        assert!(report.matches.is_empty());
        assert_eq!(report.unmatched_subtitles, [0, 1]);
    }
}
