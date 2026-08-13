//! What a subtitle track inside a container is written as.

/// The subtitle codecs a Matroska file declares, and what each of them is.
///
/// The distinction that matters is text against pictures. A text track is
/// dialogue and becomes a transcript, a search index and everything else the
/// application is for. A bitmap track is a sequence of images of dialogue, and
/// turning those into words means optical character recognition, a large
/// dependency and an accuracy figure nothing else here has to apologise for. So
/// they are recognised, named, and left alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubtitleCodec {
    /// Plain text, one cue to a block. This is SRT inside a container.
    SubRip,
    /// The format that ASS and SSA files are written in, whose blocks are
    /// comma separated records with the dialogue in the last field.
    SubStationAlpha,
    WebVtt,
    /// Presentation graphics, as Blu-ray discs carry. Pictures.
    Pgs,
    /// The subtitle format of DVDs. Pictures.
    VobSub,
    /// Broadcast subtitles. Pictures.
    DvbSub,
    /// Something this build has no name for, which is read as unsupported
    /// rather than guessed at.
    #[default]
    Unknown,
}

impl SubtitleCodec {
    /// What a codec identifier in a file header comes to.
    ///
    /// The identifiers without the `S_TEXT/` prefix are what files written
    /// before that part of the specification settled carry, and there are
    /// enough of them about to be worth two lines.
    #[must_use]
    pub fn of(codec_id: &str) -> Self {
        let id = codec_id.trim();
        if id.eq_ignore_ascii_case("S_TEXT/UTF8") || id.eq_ignore_ascii_case("S_TEXT/ASCII") {
            return Self::SubRip;
        }
        for name in ["S_TEXT/SSA", "S_TEXT/ASS", "S_SSA", "S_ASS"] {
            if id.eq_ignore_ascii_case(name) {
                return Self::SubStationAlpha;
            }
        }
        if id.eq_ignore_ascii_case("S_TEXT/WEBVTT") {
            return Self::WebVtt;
        }
        if id.eq_ignore_ascii_case("S_HDMV/PGS") {
            return Self::Pgs;
        }
        // The compressed variant differs only in how the pictures are stored,
        // and pictures are pictures either way.
        if id.eq_ignore_ascii_case("S_VOBSUB") || id.eq_ignore_ascii_case("S_VOBSUB/ZLIB") {
            return Self::VobSub;
        }
        if id.eq_ignore_ascii_case("S_DVBSUB") {
            return Self::DvbSub;
        }
        Self::Unknown
    }

    /// Whether the blocks of this track are dialogue this application can read.
    #[must_use]
    pub fn is_text(self) -> bool {
        matches!(self, Self::SubRip | Self::SubStationAlpha | Self::WebVtt)
    }

    /// Whether the blocks of this track are images of dialogue.
    #[must_use]
    pub fn is_bitmap(self) -> bool {
        matches!(self, Self::Pgs | Self::VobSub | Self::DvbSub)
    }

    /// The name this is stored and sent across the boundary under.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SubRip => "subrip",
            Self::SubStationAlpha => "ass",
            Self::WebVtt => "webvtt",
            Self::Pgs => "pgs",
            Self::VobSub => "vobsub",
            Self::DvbSub => "dvbsub",
            Self::Unknown => "unknown",
        }
    }

    /// Reads back what [`Self::as_str`] wrote.
    ///
    /// Anything else reads as unknown, so a row written by a later version
    /// naming a codec this build has never heard of is left alone rather than
    /// being taken for one it recognises.
    #[must_use]
    pub fn from_stored(text: &str) -> Self {
        match text {
            "subrip" => Self::SubRip,
            "ass" => Self::SubStationAlpha,
            "webvtt" => Self::WebVtt,
            "pgs" => Self::Pgs,
            "vobsub" => Self::VobSub,
            "dvbsub" => Self::DvbSub,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SubtitleCodec;

    #[test]
    fn recognises_the_text_codecs() {
        assert_eq!(SubtitleCodec::of("S_TEXT/UTF8"), SubtitleCodec::SubRip);
        assert_eq!(SubtitleCodec::of("S_TEXT/ASCII"), SubtitleCodec::SubRip);
        assert_eq!(
            SubtitleCodec::of("S_TEXT/ASS"),
            SubtitleCodec::SubStationAlpha
        );
        assert_eq!(SubtitleCodec::of("S_SSA"), SubtitleCodec::SubStationAlpha);
        assert_eq!(SubtitleCodec::of("S_TEXT/WEBVTT"), SubtitleCodec::WebVtt);

        for codec in [
            SubtitleCodec::SubRip,
            SubtitleCodec::SubStationAlpha,
            SubtitleCodec::WebVtt,
        ] {
            assert!(codec.is_text());
            assert!(!codec.is_bitmap());
        }
    }

    #[test]
    fn recognises_the_ones_that_are_pictures() {
        assert_eq!(SubtitleCodec::of("S_HDMV/PGS"), SubtitleCodec::Pgs);
        assert_eq!(SubtitleCodec::of("S_VOBSUB"), SubtitleCodec::VobSub);
        assert_eq!(SubtitleCodec::of("S_VOBSUB/ZLIB"), SubtitleCodec::VobSub);
        assert_eq!(SubtitleCodec::of("S_DVBSUB"), SubtitleCodec::DvbSub);

        for codec in [
            SubtitleCodec::Pgs,
            SubtitleCodec::VobSub,
            SubtitleCodec::DvbSub,
        ] {
            assert!(codec.is_bitmap());
            assert!(!codec.is_text());
        }
    }

    #[test]
    fn anything_else_is_unknown_rather_than_assumed() {
        // Both of these are text of a sort. Neither is text this build reads,
        // and claiming otherwise would put an empty transcript in front of
        // somebody.
        assert_eq!(SubtitleCodec::of("S_TEXT/USF"), SubtitleCodec::Unknown);
        assert_eq!(SubtitleCodec::of("S_HDMV/TEXTST"), SubtitleCodec::Unknown);
        assert_eq!(SubtitleCodec::of(""), SubtitleCodec::Unknown);
        assert!(!SubtitleCodec::Unknown.is_text());
        assert!(!SubtitleCodec::Unknown.is_bitmap());
    }

    #[test]
    fn a_codec_is_read_however_it_was_spelled() {
        assert_eq!(SubtitleCodec::of("s_text/utf8"), SubtitleCodec::SubRip);
        assert_eq!(SubtitleCodec::of(" S_HDMV/PGS "), SubtitleCodec::Pgs);
    }

    #[test]
    fn a_codec_survives_a_round_trip() {
        for codec in [
            SubtitleCodec::SubRip,
            SubtitleCodec::SubStationAlpha,
            SubtitleCodec::WebVtt,
            SubtitleCodec::Pgs,
            SubtitleCodec::VobSub,
            SubtitleCodec::DvbSub,
            SubtitleCodec::Unknown,
        ] {
            assert_eq!(SubtitleCodec::from_stored(codec.as_str()), codec);
        }
        assert_eq!(
            SubtitleCodec::from_stored("something later"),
            SubtitleCodec::Unknown
        );
    }
}
