//! The suffixes that say what a subtitle file is rather than which film it
//! belongs to.
//!
//! Subtitle files are conventionally named after the film with the language
//! appended, sometimes followed by a note that the track is forced or is
//! written for viewers who cannot hear the audio. Those parts have to come off
//! the name before it can be compared to a film, and they are worth keeping
//! because the library shows them.

/// What a subtitle file says about itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubtitleLabel {
    /// The two letter code for the language, where the name gave one.
    pub language: Option<&'static str>,
    /// A track meant to be shown regardless of the subtitle setting, usually
    /// for signs and for dialogue in another language.
    pub forced: bool,
    /// A track that also describes the audio.
    pub hearing_impaired: bool,
}

impl SubtitleLabel {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.language.is_none() && !self.forced && !self.hearing_impaired
    }
}

/// The two letter code first, then the other spellings that stand for it.
///
/// Only the languages that subtitle files are actually distributed in. An
/// unknown suffix is left on the name, which costs a pairing at worst,
/// whereas a wrong entry here would quietly remove part of a title.
const LANGUAGES: &[(&str, &[&str])] = &[
    ("ar", &["ara", "arabic"]),
    ("bg", &["bul", "bulgarian"]),
    ("cs", &["cze", "ces", "czech"]),
    ("da", &["dan", "danish"]),
    ("de", &["ger", "deu", "german"]),
    ("el", &["gre", "ell", "greek"]),
    ("en", &["eng", "english"]),
    ("es", &["spa", "spanish", "castellano", "latino"]),
    ("fa", &["per", "fas", "persian", "farsi"]),
    ("fi", &["fin", "finnish"]),
    ("fr", &["fre", "fra", "french"]),
    ("he", &["heb", "hebrew"]),
    ("hi", &["hin", "hindi"]),
    ("hr", &["hrv", "croatian"]),
    ("hu", &["hun", "hungarian"]),
    ("id", &["ind", "indonesian"]),
    ("it", &["ita", "italian"]),
    ("ja", &["jpn", "japanese"]),
    ("ko", &["kor", "korean"]),
    ("nl", &["dut", "nld", "dutch"]),
    ("no", &["nor", "norwegian"]),
    ("pl", &["pol", "polish"]),
    ("pt", &["por", "portuguese", "brazilian"]),
    ("ro", &["rum", "ron", "romanian"]),
    ("ru", &["rus", "russian"]),
    ("sr", &["srp", "serbian"]),
    ("sv", &["swe", "swedish"]),
    ("th", &["tha", "thai"]),
    ("tr", &["tur", "turkish"]),
    ("uk", &["ukr", "ukrainian"]),
    ("vi", &["vie", "vietnamese"]),
    ("zh", &["chi", "zho", "chinese", "mandarin", "cantonese"]),
];

/// The two letter code a name for a language stands for.
pub(crate) fn language_code(token: &str) -> Option<&'static str> {
    LANGUAGES.iter().find_map(|(code, aliases)| {
        let matches = code.eq_ignore_ascii_case(token)
            || aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(token));
        matches.then_some(*code)
    })
}

pub(crate) fn is_forced(token: &str) -> bool {
    token.eq_ignore_ascii_case("forced")
}

pub(crate) fn is_hearing_impaired(token: &str) -> bool {
    ["sdh", "cc"]
        .iter()
        .any(|name| name.eq_ignore_ascii_case(token))
}

#[cfg(test)]
mod tests {
    use super::{is_forced, is_hearing_impaired, language_code};

    #[test]
    fn reads_the_spellings_a_language_is_written_in() {
        for token in ["en", "EN", "eng", "English"] {
            assert_eq!(language_code(token), Some("en"), "{token}");
        }
        assert_eq!(language_code("pt"), Some("pt"));
        assert_eq!(language_code("brazilian"), Some("pt"));
        assert_eq!(language_code("klingon"), None);
    }

    #[test]
    fn reads_the_other_suffixes() {
        assert!(is_forced("FORCED"));
        assert!(!is_forced("force"));
        assert!(is_hearing_impaired("sdh"));
        assert!(is_hearing_impaired("CC"));
        assert!(!is_hearing_impaired("sd"));
    }
}
