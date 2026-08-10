//! Reading a file name.
//!
//! A film on disk is rarely called what the film is called. It carries the
//! resolution, where it came from, which codec it was encoded with and who
//! released it, all run together with full stops. Working out the title, the
//! year and a form that can be compared against a subtitle file is the whole
//! job of this module.

use super::language::{SubtitleLabel, is_forced, is_hearing_impaired, language_code};

/// The first year a film was released and a bound comfortably beyond any film
/// that could be sitting on a disk. Anything outside is a resolution, a track
/// number or part of a title.
const EARLIEST_YEAR: u16 = 1888;
const LATEST_YEAR: u16 = 2100;

/// Picture, source and codec markers, which end a title with certainty.
///
/// Everything from the first of these onwards is discarded, which is also how
/// the release group on the end is removed without needing a list of groups.
const STRONG_NOISE: &[&str] = &[
    "4k",
    "8k",
    "uhd",
    "fhd",
    "qhd",
    "hdr",
    "hdr10",
    "sdr",
    "10bit",
    "8bit",
    "hi10p",
    "dolbyvision",
    "bluray",
    "bdrip",
    "bdremux",
    "brrip",
    "webrip",
    "webdl",
    "hdtv",
    "pdtv",
    "dvdrip",
    "dvdscr",
    "dvd",
    "hdrip",
    "hdcam",
    "camrip",
    "telesync",
    "telecine",
    "screener",
    "vodrip",
    "hddvd",
    "remux",
    "r5",
    "x264",
    "x265",
    "h264",
    "h265",
    "avc",
    "hevc",
    "xvid",
    "divx",
    "av1",
    "vp9",
];

/// Edition and audio markers, which are junk at the end of a title but might be
/// words inside one.
///
/// These are only ever taken off the end, so a film called The Extended Family
/// keeps its name while Some Film EXTENDED loses its suffix.
const WEAK_NOISE: &[&str] = &[
    "extended",
    "unrated",
    "uncut",
    "remastered",
    "imax",
    "proper",
    "repack",
    "limited",
    "internal",
    "theatrical",
    "directors",
    "subbed",
    "dubbed",
    "retail",
    "commentary",
    "aac",
    "ac3",
    "eac3",
    "ddp",
    "dd",
    "dts",
    "dtshd",
    "truehd",
    "atmos",
    "flac",
    "opus",
    "lpcm",
    "mp3",
];

/// Language codes that are also ordinary English words, and so cannot be taken
/// off the end of a name on sight.
const AMBIGUOUS_CODES: &[&str] = &["el", "he", "hi", "id", "it", "no"];

/// Words that stay in lower case when a title has to be capitalised.
const SMALL_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "for", "from", "in", "nor", "of", "on", "or", "the",
    "to", "with",
];

/// What a file name turned out to be about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedName {
    /// The title as it should be shown.
    pub title: String,
    pub year: Option<u16>,
    /// The title reduced to letters and digits, for comparison.
    pub key: String,
    /// The same, kept as separate words, so that containment can be tested a
    /// word at a time rather than a letter at a time.
    pub key_words: Vec<String>,
    /// What the name said about the subtitle file itself. Empty for a film.
    pub label: SubtitleLabel,
}

impl ParsedName {
    #[must_use]
    pub fn from_file_name(file_name: &str) -> Self {
        let stem = strip_extension(file_name);
        let all = tokenise(stem);

        let (end, label) = read_label(&all);
        let tokens = &all[..end];

        let (title_end, year) = split_title(tokens);
        let title_tokens = &tokens[..title_end];

        if title_tokens.is_empty() {
            // Nothing recognisable, so show the name as it is rather than
            // showing nothing.
            return Self {
                title: stem.trim().to_owned(),
                year,
                key: String::new(),
                key_words: Vec::new(),
                label,
            };
        }

        let key_words: Vec<String> = title_tokens
            .iter()
            .map(|token| comparable(token.text))
            .filter(|word| !word.is_empty())
            .collect();

        Self {
            key: key_words.concat(),
            key_words,
            title: display_title(title_tokens),
            year,
            label,
        }
    }

    /// Whether two names could describe the same film as far as the year goes.
    ///
    /// A missing year agrees with anything, since plenty of files do not carry
    /// one, but two different years are a definite no.
    #[must_use]
    pub fn year_agrees_with(&self, other: &Self) -> bool {
        match (self.year, other.year) {
            (Some(mine), Some(theirs)) => mine == theirs,
            _ => true,
        }
    }
}

struct Token<'a> {
    text: &'a str,
    /// Whether a hyphen came immediately before it, so that Spider-Man can be
    /// put back together for display.
    hyphenated: bool,
}

fn strip_extension(file_name: &str) -> &str {
    match file_name.rfind('.') {
        Some(at) if at > 0 => {
            let extension = &file_name[at + 1..];
            let plausible = (1..=5).contains(&extension.len())
                && extension.bytes().all(|byte| byte.is_ascii_alphanumeric());
            if plausible {
                &file_name[..at]
            } else {
                file_name
            }
        }
        _ => file_name,
    }
}

fn tokenise(stem: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut hyphenated = false;
    let mut pending_hyphen = false;

    for (at, character) in stem.char_indices() {
        if is_separator(character) {
            if let Some(from) = start.take() {
                tokens.push(Token {
                    text: &stem[from..at],
                    hyphenated,
                });
                hyphenated = false;
            }
            pending_hyphen |= character == '-';
            continue;
        }
        if start.is_none() {
            start = Some(at);
            hyphenated = pending_hyphen;
            pending_hyphen = false;
        }
    }
    if let Some(from) = start {
        tokens.push(Token {
            text: &stem[from..],
            hyphenated,
        });
    }

    tokens
}

fn is_separator(character: char) -> bool {
    matches!(
        character,
        '.' | '_' | ' ' | '-' | '+' | '~' | ',' | '(' | ')' | '[' | ']' | '{' | '}'
    )
}

/// Takes the language and accessibility suffixes off the end of a name.
///
/// Returns where the rest of the name stops. The scan runs backwards because
/// these suffixes are always last, which is also what keeps a title ending in
/// a word like It from being mistaken for a language code.
fn read_label(tokens: &[Token<'_>]) -> (usize, SubtitleLabel) {
    let mut end = tokens.len();
    let mut label = SubtitleLabel::default();

    while end > 0 {
        let token = tokens[end - 1].text;
        if is_forced(token) {
            label.forced = true;
        } else if is_hearing_impaired(token) {
            label.hearing_impaired = true;
        } else if let Some(code) = language_code(token) {
            // Some two letter codes are also English words, so those need more
            // of a title to be left behind before the suffix can be believed.
            // This is what keeps Doctor No.srt from becoming Doctor while
            // still reading the suffix in Up.en.srt.
            let ambiguous = token.len() == 2 && AMBIGUOUS_CODES.contains(&code);
            if end >= if ambiguous { 3 } else { 2 } {
                label.language = Some(code);
            } else {
                break;
            }
        } else {
            break;
        }
        end -= 1;
    }

    (end, label)
}

/// Finds where the title stops and the year, if the name carries one.
fn split_title(tokens: &[Token<'_>]) -> (usize, Option<u16>) {
    // The first token is never junk: a film may well be called 2012 or Extended.
    let cut = (1..tokens.len())
        .find(|&at| year_of(tokens[at].text).is_some() || is_strong_noise(tokens, at));

    let Some(cut) = cut else {
        return (trim_weak_noise(tokens, tokens.len()), None);
    };

    // A run of years at the cut means the earlier ones are part of the title,
    // as in Blade Runner 2049 2017. The last is the release year.
    let mut year = None;
    let mut at = cut;
    while at < tokens.len()
        && let Some(candidate) = year_of(tokens[at].text)
    {
        year = Some((at, candidate));
        at += 1;
    }

    let (title_end, year) = match year {
        Some((at, year)) => (at, Some(year)),
        // The name was cut by something else, so a year may still be further
        // along, as in Some Film 1080p 2019 BluRay.
        None => (
            cut,
            tokens[cut..].iter().find_map(|token| year_of(token.text)),
        ),
    };

    (trim_weak_noise(tokens, title_end), year)
}

fn trim_weak_noise(tokens: &[Token<'_>], mut end: usize) -> usize {
    while end > 1
        && WEAK_NOISE
            .iter()
            .any(|noise| noise.eq_ignore_ascii_case(tokens[end - 1].text))
    {
        end -= 1;
    }
    end
}

fn year_of(token: &str) -> Option<u16> {
    if token.len() != 4 || !token.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    token
        .parse()
        .ok()
        .filter(|year| (EARLIEST_YEAR..=LATEST_YEAR).contains(year))
}

fn is_strong_noise(tokens: &[Token<'_>], at: usize) -> bool {
    let token = tokens[at].text;
    if STRONG_NOISE
        .iter()
        .any(|noise| noise.eq_ignore_ascii_case(token))
        || is_resolution(token)
    {
        return true;
    }

    // Web-DL, WEBRip and Blu-ray arrive as two tokens once the hyphen has been
    // split on, and neither half is junk on its own.
    let next = tokens.get(at + 1).map(|token| token.text);
    if token.eq_ignore_ascii_case("web") {
        return next.is_some_and(|next| {
            next.eq_ignore_ascii_case("dl") || next.eq_ignore_ascii_case("rip")
        });
    }
    if token.eq_ignore_ascii_case("blu") {
        return next.is_some_and(|next| next.eq_ignore_ascii_case("ray"));
    }
    false
}

fn is_resolution(token: &str) -> bool {
    token.len() >= 3
        && token.ends_with(['p', 'P'])
        && token[..token.len() - 1]
            .bytes()
            .all(|byte| byte.is_ascii_digit())
}

/// Reduces a word to the letters and digits it shares with the same word
/// written by someone else.
fn comparable(token: &str) -> String {
    token
        .chars()
        .flat_map(char::to_lowercase)
        .map(fold_accent)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

/// Folds the accents that the same title is written with and without.
fn fold_accent(character: char) -> char {
    match character {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'æ' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        'ß' => 's',
        _ => character,
    }
}

fn display_title(tokens: &[Token<'_>]) -> String {
    let mut title = String::new();
    for (at, token) in tokens.iter().enumerate() {
        if at > 0 {
            title.push(if token.hyphenated { '-' } else { ' ' });
        }
        title.push_str(token.text);
    }

    // Names written entirely in lower case are common and read badly in the
    // library. Names that already carry capitals are left alone, since a title
    // may well know better than a rule does.
    if title.bytes().any(|byte| byte.is_ascii_uppercase()) {
        title
    } else {
        capitalise(&title)
    }
}

fn capitalise(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut first = true;

    for part in title.split_inclusive([' ', '-']) {
        let (word, separator) = match part.char_indices().next_back() {
            Some((at, last)) if last == ' ' || last == '-' => (&part[..at], &part[at..]),
            _ => (part, ""),
        };

        if !first
            && SMALL_WORDS
                .iter()
                .any(|small| small.eq_ignore_ascii_case(word))
        {
            out.push_str(word);
        } else {
            let mut characters = word.chars();
            if let Some(initial) = characters.next() {
                out.extend(initial.to_uppercase());
                out.push_str(characters.as_str());
            }
        }
        out.push_str(separator);
        first = false;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::ParsedName;

    fn parse(file_name: &str) -> ParsedName {
        ParsedName::from_file_name(file_name)
    }

    #[test]
    fn reads_a_scene_name() {
        let name = parse("The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv");
        assert_eq!(name.title, "The Matrix");
        assert_eq!(name.year, Some(1999));
        assert_eq!(name.key, "thematrix");
        assert_eq!(name.key_words, ["the", "matrix"]);
    }

    #[test]
    fn reads_a_plainly_named_file() {
        let name = parse("The Matrix (1999).mp4");
        assert_eq!(name.title, "The Matrix");
        assert_eq!(name.year, Some(1999));
        assert_eq!(name.key, "thematrix");
    }

    #[test]
    fn a_name_with_no_year_or_junk_is_left_alone() {
        let name = parse("Some Short Film.mkv");
        assert_eq!(name.title, "Some Short Film");
        assert_eq!(name.year, None);
        assert_eq!(name.key, "someshortfilm");
    }

    #[test]
    fn keeps_a_number_that_belongs_to_the_title() {
        let name = parse("Blade.Runner.2049.2017.2160p.UHD.BluRay.x265-GROUP.mkv");
        assert_eq!(name.title, "Blade Runner 2049");
        assert_eq!(name.year, Some(2017));

        let only_a_year = parse("2012.2009.720p.BluRay.x264.mkv");
        assert_eq!(only_a_year.title, "2012");
        assert_eq!(only_a_year.year, Some(2009));
    }

    #[test]
    fn keeps_a_hyphen_that_belongs_to_the_title() {
        let name = parse("Spider-Man.Into.the.Spider-Verse.2018.1080p.WEB-DL.mkv");
        assert_eq!(name.title, "Spider-Man Into the Spider-Verse");
        assert_eq!(name.year, Some(2018));
        assert_eq!(name.key, "spidermanintothespiderverse");
    }

    #[test]
    fn takes_the_edition_off_the_end_but_not_out_of_the_middle() {
        assert_eq!(parse("Some.Film.EXTENDED.1080p.mkv").title, "Some Film");
        assert_eq!(
            parse("The.Extended.Family.2024.1080p.mkv").title,
            "The Extended Family"
        );
        assert_eq!(parse("Some Film REMASTERED.mkv").title, "Some Film");
    }

    #[test]
    fn recognises_the_sources_that_are_written_with_a_hyphen() {
        assert_eq!(parse("Some.Film.2019.WEB-DL.1080p.mkv").title, "Some Film");
        assert_eq!(parse("Some.Film.Blu-ray.mkv").title, "Some Film");
        // The word itself is not junk when it stands alone.
        assert_eq!(parse("Web of Lies.mkv").title, "Web of Lies");
    }

    #[test]
    fn capitalises_a_name_written_in_lower_case() {
        assert_eq!(parse("the.matrix.1999.mkv").title, "The Matrix");
        assert_eq!(
            parse("a.tale.of.two.cities.mkv").title,
            "A Tale of Two Cities"
        );
        // A name that already has capitals is trusted as it is.
        assert_eq!(parse("iRobot.2004.mkv").title, "iRobot");
    }

    #[test]
    fn folds_accents_for_comparison_but_not_for_display() {
        let name = parse("Amélie.2001.1080p.mkv");
        assert_eq!(name.title, "Amélie");
        assert_eq!(name.key, "amelie");
    }

    #[test]
    fn reads_the_subtitle_suffixes() {
        let plain = parse("The.Matrix.1999.1080p.BluRay.en.srt");
        assert_eq!(plain.title, "The Matrix");
        assert_eq!(plain.label.language, Some("en"));
        assert!(!plain.label.forced);

        let forced = parse("The Matrix.en.forced.srt");
        assert_eq!(forced.title, "The Matrix");
        assert_eq!(forced.label.language, Some("en"));
        assert!(forced.label.forced);

        let described = parse("The Matrix.English.SDH.srt");
        assert_eq!(described.title, "The Matrix");
        assert_eq!(described.label.language, Some("en"));
        assert!(described.label.hearing_impaired);
    }

    #[test]
    fn a_short_title_is_not_mistaken_for_a_language() {
        // Italian is "it", so a two token name ending in It has to be left
        // alone or the title disappears.
        let name = parse("The.It.srt");
        assert_eq!(name.title, "The It");
        assert_eq!(name.label.language, None);

        let with_language = parse("The.Italian.Job.it.srt");
        assert_eq!(with_language.title, "The Italian Job");
        assert_eq!(with_language.label.language, Some("it"));

        // No is Norwegian, which would otherwise take half the title away.
        let doctor = parse("Doctor No.srt");
        assert_eq!(doctor.title, "Doctor No");
        assert_eq!(doctor.label.language, None);

        // A code that is not also an English word needs no such caution.
        let short = parse("Up.en.srt");
        assert_eq!(short.title, "Up");
        assert_eq!(short.label.language, Some("en"));
    }

    #[test]
    fn years_agree_unless_they_are_both_present_and_different() {
        let with = parse("Some.Film.1999.mkv");
        let also_with = parse("Some.Film.1999.srt");
        let other = parse("Some.Film.2003.srt");
        let without = parse("Some.Film.srt");

        assert!(with.year_agrees_with(&also_with));
        assert!(with.year_agrees_with(&without));
        assert!(without.year_agrees_with(&other));
        assert!(!with.year_agrees_with(&other));
    }

    #[test]
    fn copes_with_names_that_are_barely_names() {
        for file_name in ["", ".srt", "...", "1080p.mkv", "----"] {
            let name = parse(file_name);
            assert!(name.year.is_none(), "{file_name}");
        }
        assert_eq!(parse("1080p.mkv").title, "1080p");
    }
}
