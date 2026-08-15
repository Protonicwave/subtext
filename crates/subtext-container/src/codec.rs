//! What a track inside a container is written as.
//!
//! Subtitles get a type of their own, because what they are written as decides
//! whether this application can read them at all. Picture and sound get a name
//! and nothing more: nothing here decodes either of them, so the only question
//! ever asked is what to call the codec on a screen.

/// The subtitle codecs a Matroska file declares, and what each of them is.
///
/// The distinction that matters is text against pictures. A text track is
/// dialogue, and dialogue is what this application reads. A bitmap track is a
/// sequence of images of dialogue, and
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

/// What the codecs films are encoded in are called, by the identifier a
/// Matroska header writes them under.
///
/// Matched by prefix, since a family writes its variants as further path
/// segments and every one of them is the same codec as far as a person reading
/// a screen is concerned. Longest first, so that the more specific of an
/// overlapping pair wins.
const VIDEO_NAMES: &[(&str, &str)] = &[
    ("V_MPEGH/ISO/HEVC", "HEVC"),
    ("V_MPEG4/ISO/AVC", "H.264"),
    ("V_MPEG4/ISO/ASP", "MPEG-4"),
    ("V_MPEG2", "MPEG-2"),
    ("V_MPEG1", "MPEG-1"),
    ("V_AV1", "AV1"),
    ("V_VP9", "VP9"),
    ("V_VP8", "VP8"),
    ("V_THEORA", "Theora"),
    ("V_PRORES", "ProRes"),
];

const AUDIO_NAMES: &[(&str, &str)] = &[
    ("A_MPEG/L3", "MP3"),
    ("A_MPEG/L2", "MP2"),
    ("A_TRUEHD", "TrueHD"),
    ("A_VORBIS", "Vorbis"),
    ("A_EAC3", "E-AC-3"),
    ("A_OPUS", "Opus"),
    ("A_FLAC", "FLAC"),
    ("A_ALAC", "ALAC"),
    ("A_AC3", "AC-3"),
    ("A_DTS", "DTS"),
    ("A_AAC", "AAC"),
    ("A_PCM", "PCM"),
];

/// What to call the codec a film's picture is in, where it is one with a name.
///
/// Nothing else in the application acts on this, so an identifier that is not
/// in the list is not a problem to solve: what the file said is shown instead,
/// which is more use to somebody than the word unknown.
#[must_use]
pub fn video_codec_name(codec_id: &str) -> Option<&'static str> {
    name_of(VIDEO_NAMES, codec_id)
}

/// The same, for the codec a film's sound is in.
#[must_use]
pub fn audio_codec_name(codec_id: &str) -> Option<&'static str> {
    name_of(AUDIO_NAMES, codec_id)
}

fn name_of(names: &[(&str, &'static str)], codec_id: &str) -> Option<&'static str> {
    let id = codec_id.trim();
    names.iter().find_map(|(prefix, name)| {
        let head = id.get(..prefix.len())?;
        head.eq_ignore_ascii_case(prefix).then_some(*name)
    })
}

#[cfg(test)]
mod tests {
    use super::{SubtitleCodec, audio_codec_name, video_codec_name};

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
        // and claiming otherwise would offer somebody a track with nothing in
        // it.
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

    #[test]
    fn a_picture_codec_is_named_by_the_family_it_belongs_to() {
        assert_eq!(video_codec_name("V_MPEG4/ISO/AVC"), Some("H.264"));
        assert_eq!(video_codec_name("V_MPEGH/ISO/HEVC"), Some("HEVC"));
        assert_eq!(video_codec_name("V_AV1"), Some("AV1"));
        assert_eq!(video_codec_name("v_vp9"), Some("VP9"));

        // The two that begin the same way, where the longer must not be read as
        // the shorter.
        assert_ne!(
            video_codec_name("V_MPEG4/ISO/AVC"),
            video_codec_name("V_MPEG4/ISO/ASP")
        );

        // A codec nobody here has a name for, which is shown as the file wrote
        // it rather than as nothing.
        assert_eq!(video_codec_name("V_MS/VFW/FOURCC"), None);
        assert_eq!(video_codec_name(""), None);
    }

    #[test]
    fn a_sound_codec_is_named_the_way_a_person_would_say_it() {
        assert_eq!(audio_codec_name("A_AAC"), Some("AAC"));
        // The variants a muxer writes, which are all the same codec to a reader.
        assert_eq!(audio_codec_name("A_AAC/MPEG4/LC/SBR"), Some("AAC"));
        assert_eq!(audio_codec_name("A_DTS/MA"), Some("DTS"));
        assert_eq!(audio_codec_name("A_PCM/INT/LIT"), Some("PCM"));

        assert_eq!(audio_codec_name("A_AC3"), Some("AC-3"));
        assert_eq!(audio_codec_name("A_EAC3"), Some("E-AC-3"));
        assert_eq!(audio_codec_name("A_MPEG/L3"), Some("MP3"));

        assert_eq!(audio_codec_name("A_REAL/14_4"), None);
    }
}
