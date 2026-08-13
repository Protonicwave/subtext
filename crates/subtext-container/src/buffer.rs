//! Reading a file in windows rather than a byte at a time.
//!
//! Finding the tracks in a header costs a few hundred bytes and does not care
//! how they are read. Reading the dialogue out of one is a different shape of
//! work: a film is a long chain of small elements, and the reader steps over
//! almost all of them, taking a few bytes of each to learn what it is. Done
//! directly against a file that is one system call per byte, and a two hour
//! film holds around half a million blocks.
//!
//! So a window of the file is held, reads come out of it, and a seek that lands
//! inside it costs nothing at all. That is what makes stepping over a sound
//! frame free while stepping over a picture frame, which is larger than the
//! window, still costs the seek it has to.

use std::io::{Read, Seek, SeekFrom};

/// How much is taken at the first read after a seek.
///
/// Enough for an element's header and the first bytes of its payload, which
/// together are what says whether the rest of it is wanted. Taking more would
/// mean reading part of the frame the seek was there to step over.
const MIN_WINDOW: usize = 512;

/// How much is taken once reads have proved to be running on.
///
/// A run of small elements, which is what a header is and what the inside of a
/// cluster is between one frame and the next, is then a few reads rather than
/// one per element.
const MAX_WINDOW: usize = 64 << 10;

/// A source with a window of it held in memory.
#[derive(Debug)]
pub(crate) struct Buffered<R> {
    source: R,
    /// The bytes held, and where in the source they begin.
    held: Vec<u8>,
    from: u64,
    /// Where the next read comes from, which is not where the source is.
    at: u64,
    /// Where the source itself is, so that a read already in the right place
    /// does not seek to where it already is.
    source_at: u64,
    /// How much the next fill takes.
    window: usize,
}

impl<R: Read + Seek> Buffered<R> {
    pub(crate) fn new(source: R) -> Self {
        Self {
            source,
            held: Vec::new(),
            from: 0,
            at: 0,
            source_at: 0,
            window: MIN_WINDOW,
        }
    }

    fn holds(&self, at: u64) -> Option<usize> {
        let offset = at.checked_sub(self.from)?;
        let offset = usize::try_from(offset).ok()?;
        (offset < self.held.len()).then_some(offset)
    }

    /// Puts the source where the next read is to come from.
    fn place(&mut self) -> std::io::Result<()> {
        if self.source_at != self.at {
            self.source.seek(SeekFrom::Start(self.at))?;
            self.source_at = self.at;
        }
        Ok(())
    }

    fn fill(&mut self) -> std::io::Result<()> {
        // A fill carrying on from where the last one ended is a run of reads,
        // and the window grows so that the dense stretches of a file cost a few
        // reads rather than one per element. Anything else is a step over a
        // frame, and there the window goes back to its smallest, since a large
        // one would read the frame this is trying not to read.
        let running_on = self.at == self.from.saturating_add(self.held.len() as u64);
        self.window = if running_on && !self.held.is_empty() {
            self.window.saturating_mul(2).min(MAX_WINDOW)
        } else {
            MIN_WINDOW
        };

        self.place()?;
        self.held.clear();
        self.held.resize(self.window, 0);

        let read = self.source.read(&mut self.held)?;
        self.held.truncate(read);
        self.from = self.at;
        self.source_at = self.at.saturating_add(read as u64);
        Ok(())
    }
}

impl<R: Read + Seek> Read for Buffered<R> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }

        // A read as large as the window would fill it and then be copied out of
        // it, which is the same bytes moved twice.
        if out.len() >= MAX_WINDOW {
            self.place()?;
            let read = self.source.read(out)?;
            self.at = self.at.saturating_add(read as u64);
            self.source_at = self.at;
            return Ok(read);
        }

        if self.holds(self.at).is_none() {
            self.fill()?;
        }
        // Still nothing to give, which is the end of the file.
        let Some(offset) = self.holds(self.at) else {
            return Ok(0);
        };

        let taken = out.len().min(self.held.len() - offset);
        out[..taken].copy_from_slice(&self.held[offset..offset + taken]);
        self.at = self.at.saturating_add(taken as u64);
        Ok(taken)
    }
}

impl<R: Read + Seek> Seek for Buffered<R> {
    /// Moves where the next read comes from, without moving the source.
    ///
    /// This is the point of the whole thing. Stepping over an element is a seek,
    /// and where it lands inside the window the bytes are already here.
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.at = match to {
            SeekFrom::Start(at) => at,
            SeekFrom::Current(by) => self.at.saturating_add_signed(by),
            // Only the source knows where its end is.
            SeekFrom::End(back) => {
                let end = self.source.seek(SeekFrom::End(back))?;
                self.source_at = end;
                end
            }
        };
        Ok(self.at)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::cast_possible_truncation, clippy::unwrap_used)]

    use std::io::{Cursor, Read, Seek, SeekFrom};

    use super::{Buffered, MAX_WINDOW};

    /// A source that counts what was asked of it.
    #[derive(Debug)]
    struct Counted {
        bytes: Cursor<Vec<u8>>,
        reads: usize,
        taken: u64,
        seeks: usize,
    }

    impl Read for Counted {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            let read = self.bytes.read(out)?;
            self.taken += read as u64;
            Ok(read)
        }
    }

    impl Seek for Counted {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            self.seeks += 1;
            self.bytes.seek(to)
        }
    }

    fn source(length: usize) -> Buffered<Counted> {
        let bytes = (0..length).map(|at| at as u8).collect();
        Buffered::new(Counted {
            bytes: Cursor::new(bytes),
            reads: 0,
            taken: 0,
            seeks: 0,
        })
    }

    fn byte(reader: &mut Buffered<Counted>) -> u8 {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).unwrap();
        byte[0]
    }

    #[test]
    fn a_run_of_small_reads_costs_a_handful() {
        let mut reader = source(MAX_WINDOW * 2);
        for at in 0..MAX_WINDOW {
            assert_eq!(byte(&mut reader), at as u8);
        }

        // The window doubles as the reads keep coming, so a run of this length
        // costs the few reads it takes to get there rather than one per byte.
        assert!(reader.source.reads < 10, "{} reads", reader.source.reads);
    }

    /// The behaviour the whole thing exists for: a walk that reads a few bytes
    /// of each element and steps over the rest must not end up reading the file.
    #[test]
    fn stepping_over_frames_does_not_read_them() {
        const FRAME: usize = 24 << 10;

        let length = FRAME * 200;
        let mut reader = source(length);
        for frame in 0..200 {
            reader
                .seek(SeekFrom::Start((frame * FRAME) as u64))
                .unwrap();
            let mut header = [0u8; 8];
            reader.read_exact(&mut header).unwrap();
        }

        let read = reader.source.taken;
        assert!(
            read * 8 < length as u64,
            "{read} bytes read of {length} stepped over"
        );
    }

    #[test]
    fn a_seek_inside_the_window_costs_nothing() {
        let mut reader = source(MAX_WINDOW * 2);
        assert_eq!(byte(&mut reader), 0);

        reader.seek(SeekFrom::Start(500)).unwrap();
        assert_eq!(byte(&mut reader), 500u32 as u8);
        // Backwards as well, which is what reading an element's header and then
        // its payload does.
        reader.seek(SeekFrom::Start(200)).unwrap();
        assert_eq!(byte(&mut reader), 200u32 as u8);

        assert_eq!(reader.source.reads, 1);
        assert_eq!(reader.source.seeks, 0);
    }

    #[test]
    fn a_seek_past_the_window_goes_to_the_source() {
        let mut reader = source(MAX_WINDOW * 4);
        assert_eq!(byte(&mut reader), 0);

        let far = (MAX_WINDOW * 3) as u64;
        reader.seek(SeekFrom::Start(far)).unwrap();
        assert_eq!(byte(&mut reader), far as u8);
        assert_eq!(reader.source.reads, 2);
        assert_eq!(reader.source.seeks, 1);
    }

    #[test]
    fn a_read_larger_than_the_window_is_handed_straight_through() {
        let mut reader = source(MAX_WINDOW * 4);
        let mut out = vec![0; MAX_WINDOW * 2];
        reader.read_exact(&mut out).unwrap();

        assert_eq!(out[0], 0);
        assert_eq!(out[MAX_WINDOW * 2 - 1], (MAX_WINDOW * 2 - 1) as u8);
        assert_eq!(byte(&mut reader), (MAX_WINDOW * 2) as u8);
    }

    #[test]
    fn the_end_of_the_source_reads_as_nothing_rather_than_as_an_error() {
        let mut reader = source(4);
        let mut out = [0u8; 8];
        assert_eq!(reader.read(&mut out).unwrap(), 4);
        assert_eq!(reader.read(&mut out).unwrap(), 0);

        // And where the end is, which is what a reader asks first of all.
        assert_eq!(reader.seek(SeekFrom::End(0)).unwrap(), 4);
    }
}
