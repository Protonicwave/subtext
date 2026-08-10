//! Turning the bytes of a subtitle file into text.

use chardetng::{Iso2022JpDetection, Utf8Detection};
use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE};

/// How much of a file the encoding is guessed from. Comfortably more than the
/// few hundred bytes it takes to tell one single byte encoding from another.
const DETECTION_WINDOW: usize = 16 * 1024;

pub(crate) struct Decoded {
    pub text: String,
    pub encoding: &'static str,
    /// Whether any byte could not be decoded and was replaced.
    pub replaced: bool,
}

/// Reads the bytes of a subtitle file as text.
///
/// Subtitle files carry no encoding declaration, so this works through the
/// evidence in order: a byte order mark, then the shape of a file that is
/// clearly sixteen bit, then valid UTF-8, and only then a guess. Guessing is
/// last because it is the only step that can be wrong.
pub(crate) fn decode(bytes: &[u8]) -> Decoded {
    if let Some((encoding, mark)) = Encoding::for_bom(bytes) {
        return finish(encoding, &bytes[mark..]);
    }
    if let Some(encoding) = sixteen_bit_without_mark(bytes) {
        return finish(encoding, bytes);
    }
    if let Ok(text) = core::str::from_utf8(bytes) {
        return Decoded {
            text: text.to_owned(),
            encoding: UTF_8.name(),
            replaced: false,
        };
    }

    // The detector is told UTF-8 is not a possible answer, because the check
    // above has already ruled it out. Japanese mail encodings are allowed: the
    // warning against them concerns pages that run scripts, and this is a
    // subtitle file.
    let mut detector = chardetng::EncodingDetector::new(Iso2022JpDetection::Allow);
    // Detection is by far the most expensive step in reading a file, and it
    // costs the same per byte whether the answer was settled in the first
    // paragraph or the last. A subtitle file does not change language part way
    // through, so the opening is enough to decide on.
    detector.feed(&bytes[..bytes.len().min(DETECTION_WINDOW)], true);
    finish(detector.guess(None, Utf8Detection::Deny), bytes)
}

fn finish(encoding: &'static Encoding, bytes: &[u8]) -> Decoded {
    let (text, replaced) = encoding.decode_without_bom_handling(bytes);
    Decoded {
        text: text.into_owned(),
        encoding: encoding.name(),
        replaced,
    }
}

/// Recognises UTF-16 that was saved without a byte order mark.
///
/// This has to run before the UTF-8 check rather than after it: Latin text in
/// UTF-16 is a run of alternating characters and zero bytes, which is valid
/// UTF-8, so the UTF-8 check would accept it and produce text full of nulls.
fn sixteen_bit_without_mark(bytes: &[u8]) -> Option<&'static Encoding> {
    let sample = &bytes[..bytes.len().min(1024)];
    if sample.len() < 4 {
        return None;
    }

    let mut leading = 0;
    let mut trailing = 0;
    for (position, byte) in sample.iter().enumerate() {
        if *byte == 0 {
            if position % 2 == 0 {
                leading += 1;
            } else {
                trailing += 1;
            }
        }
    }

    // Latin text fills half the file with zeroes, all on the same side. A
    // quarter is a wide margin below that and well above anything a single
    // byte encoding would produce.
    let threshold = sample.len() / 4;
    if leading > threshold {
        Some(UTF_16BE)
    } else if trailing > threshold {
        Some(UTF_16LE)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn reads_plain_utf8() {
        let decoded = decode("Ça va".as_bytes());
        assert_eq!(decoded.text, "Ça va");
        assert_eq!(decoded.encoding, "UTF-8");
        assert!(!decoded.replaced);
    }

    #[test]
    fn strips_a_utf8_byte_order_mark() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"1");
        assert_eq!(decode(&bytes).text, "1");
    }

    #[test]
    fn reads_utf16_with_a_byte_order_mark() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "hello".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let decoded = decode(&bytes);
        assert_eq!(decoded.text, "hello");
        assert_eq!(decoded.encoding, "UTF-16LE");
    }

    #[test]
    fn reads_utf16_saved_without_a_byte_order_mark() {
        let source = "one two three four";
        let mut little = Vec::new();
        let mut big = Vec::new();
        for unit in source.encode_utf16() {
            little.extend_from_slice(&unit.to_le_bytes());
            big.extend_from_slice(&unit.to_be_bytes());
        }
        assert_eq!(decode(&little).text, source);
        assert_eq!(decode(&big).text, source);
    }

    #[test]
    fn falls_back_to_a_guess_for_single_byte_encodings() {
        // Windows-1252 for "Voilà, un café à Paris, très bien."
        let bytes = b"Voil\xE0, un caf\xE9 \xE0 Paris, tr\xE8s bien.";
        let decoded = decode(bytes);
        assert_eq!(decoded.text, "Voilà, un café à Paris, très bien.");
        assert!(!decoded.replaced);
    }

    #[test]
    fn never_fails_on_arbitrary_bytes() {
        let bytes: Vec<u8> = (0..=255).collect();
        assert!(!decode(&bytes).text.is_empty());
    }
}
