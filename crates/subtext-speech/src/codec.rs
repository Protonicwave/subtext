//! Saying which codec, in the words somebody would use for it.
//!
//! A refusal is only any use if it names what was refused. "This film's audio
//! cannot be read" leaves somebody nowhere; "this film's audio is DTS, which
//! Subtext cannot read" tells them it is the file and not the application, and
//! that no amount of trying again will change it.

use symphonia::core::codecs::audio::{AudioCodecId, well_known};

/// What to call a codec.
///
/// Only the codecs a film's audio actually arrives in are here. The list is
/// short because the set of codecs used to carry a soundtrack is short, and
/// anything outside it is rare enough that a name for it would be a guess.
pub(crate) fn name_of(codec: AudioCodecId) -> Option<&'static str> {
    let name = match codec {
        well_known::CODEC_ID_AAC => "AAC",
        well_known::CODEC_ID_MP3 => "MP3",
        well_known::CODEC_ID_MP2 => "MP2",
        well_known::CODEC_ID_MP1 => "MP1",
        well_known::CODEC_ID_FLAC => "FLAC",
        well_known::CODEC_ID_VORBIS => "Vorbis",
        well_known::CODEC_ID_ALAC => "ALAC",
        well_known::CODEC_ID_OPUS => "Opus",
        well_known::CODEC_ID_AC3 => "AC-3",
        well_known::CODEC_ID_EAC3 => "E-AC-3",
        well_known::CODEC_ID_AC4 => "AC-4",
        well_known::CODEC_ID_DCA => "DTS",
        well_known::CODEC_ID_TRUEHD => "TrueHD",
        well_known::CODEC_ID_WMA => "WMA",
        well_known::CODEC_ID_PCM_S16LE
        | well_known::CODEC_ID_PCM_S16BE
        | well_known::CODEC_ID_PCM_S24LE
        | well_known::CODEC_ID_PCM_S32LE
        | well_known::CODEC_ID_PCM_F32LE => "PCM",
        _ => return None,
    };
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::name_of;
    use symphonia::core::codecs::audio::{AudioCodecId, well_known};

    #[test]
    fn the_codecs_a_disc_rip_carries_have_names() {
        assert_eq!(name_of(well_known::CODEC_ID_AC3), Some("AC-3"));
        assert_eq!(name_of(well_known::CODEC_ID_EAC3), Some("E-AC-3"));
        assert_eq!(name_of(well_known::CODEC_ID_DCA), Some("DTS"));
        assert_eq!(name_of(well_known::CODEC_ID_TRUEHD), Some("TrueHD"));
    }

    #[test]
    fn a_codec_nothing_is_distributed_in_has_no_name_to_offer() {
        assert_eq!(name_of(AudioCodecId::default()), None);
    }
}
