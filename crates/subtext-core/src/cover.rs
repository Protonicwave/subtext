//! Where a film's cover came from.
//!
//! A path on its own draws a tile and says nothing true about it. It cannot say
//! whether somebody picked the image or whether a scan guessed at it, and
//! without that a rescan has no way of telling a choice from a guess when it
//! finds a second candidate.
//!
//! The source is the missing half. It is stored beside the path, it is carried
//! to the screen, and it is what deciding a cover compares.

use std::path::PathBuf;

/// How good a claim an image has on a film, best first.
///
/// The order is the rule the scan follows, and it is an ordering rather than a
/// list of cases because deciding between two candidates is then comparing two
/// values. The claim it represents, read from the top down: an image somebody
/// picked for this film outranks everything, because it is the only one nobody
/// guessed at; artwork inside the file comes next, since whoever made the file
/// attached it to this film and no other; an image in the film's own folder is
/// the same claim made one step further away; a path read out of a file another
/// tool wrote is that tool's answer rather than the reader's; an image in the
/// folder above stands for every film in it and so says the least of any of
/// them; and nothing at all is the honest admission that nobody chose anything.
///
/// Derived rather than written out, so a value added in the middle of the list
/// is a value added in the middle of the list and nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoverSource {
    /// Somebody picked this image for this film. No scan may overwrite it.
    Chosen,
    /// Artwork attached inside the film's own container.
    InFile,
    /// An image beside the film, or under a fixed name in its own folder.
    Beside,
    /// A path read out of a sidecar another tool wrote.
    Sidecar,
    /// An image in the folder above, which serves every film filed under it.
    FolderAbove,
    /// Nothing was found, and the tile is drawn from the film itself.
    #[default]
    Nothing,
}

impl CoverSource {
    /// What a row records, which is a name rather than a number so that reading
    /// the table says something and so that the order here can change without
    /// every stored row meaning something else.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chosen => "chosen",
            Self::InFile => "in-file",
            Self::Beside => "beside",
            Self::Sidecar => "sidecar",
            Self::FolderAbove => "folder-above",
            Self::Nothing => "none",
        }
    }

    /// Reads back what [`Self::as_str`] wrote.
    ///
    /// Anything else reads as nothing found, which is the answer that puts a
    /// film in the way of the next scan deciding afresh. A row that cannot be
    /// understood is better sent back round than trusted.
    #[must_use]
    pub fn from_stored(text: &str) -> Self {
        match text {
            "chosen" => Self::Chosen,
            "in-file" => Self::InFile,
            "beside" => Self::Beside,
            "sidecar" => Self::Sidecar,
            "folder-above" => Self::FolderAbove,
            _ => Self::Nothing,
        }
    }

    /// Whether somebody picked this cover, which is what a scan must not undo.
    #[must_use]
    pub fn is_chosen(self) -> bool {
        matches!(self, Self::Chosen)
    }
}

/// One image and the claim it has on a film.
///
/// The two travel together because neither is worth much alone: a path with no
/// source cannot be compared with another candidate, and a source with no path
/// draws nothing. [`CoverSource::Nothing`] describes a film that has no cover,
/// which is the absence of one of these rather than one of these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cover {
    pub path: PathBuf,
    pub source: CoverSource,
}

impl Cover {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, source: CoverSource) -> Self {
        Self {
            path: path.into(),
            source,
        }
    }

    /// The source of a cover a film may not have, where not having one is
    /// itself an answer.
    #[must_use]
    pub fn source_of(cover: Option<&Self>) -> CoverSource {
        cover.map_or(CoverSource::Nothing, |cover| cover.source)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cover, CoverSource};

    /// The ordering is the claim, so every pair a scan compares is asserted
    /// here rather than left to the order the variants happen to be written in.
    #[test]
    fn a_chosen_cover_outranks_everything_and_nothing_ranks_last() {
        let order = [
            CoverSource::Chosen,
            CoverSource::InFile,
            CoverSource::Beside,
            CoverSource::Sidecar,
            CoverSource::FolderAbove,
            CoverSource::Nothing,
        ];

        for (at, better) in order.iter().enumerate() {
            for worse in &order[at + 1..] {
                assert!(better < worse, "{better:?} should outrank {worse:?}");
            }
        }

        assert_eq!(order.iter().copied().min(), Some(CoverSource::Chosen));
        assert_eq!(order.iter().copied().max(), Some(CoverSource::Nothing));
    }

    #[test]
    fn every_source_survives_being_written_down_and_read_back() {
        for source in [
            CoverSource::Chosen,
            CoverSource::InFile,
            CoverSource::Beside,
            CoverSource::Sidecar,
            CoverSource::FolderAbove,
            CoverSource::Nothing,
        ] {
            assert_eq!(CoverSource::from_stored(source.as_str()), source);
        }
    }

    #[test]
    fn a_row_nobody_can_read_says_nothing_was_found() {
        assert_eq!(CoverSource::from_stored(""), CoverSource::Nothing);
        assert_eq!(CoverSource::from_stored("frame"), CoverSource::Nothing);
        assert_eq!(CoverSource::default(), CoverSource::Nothing);
    }

    #[test]
    fn a_film_with_no_cover_has_a_source_all_the_same() {
        let chosen = Cover::new("/films/Heat.jpg", CoverSource::Chosen);
        assert_eq!(Cover::source_of(Some(&chosen)), CoverSource::Chosen);
        assert!(Cover::source_of(Some(&chosen)).is_chosen());
        assert_eq!(Cover::source_of(None), CoverSource::Nothing);
        assert!(!CoverSource::Nothing.is_chosen());
    }
}
