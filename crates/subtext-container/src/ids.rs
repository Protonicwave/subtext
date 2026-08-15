//! The elements this reads, as the specification numbers them.
//!
//! Identifiers keep the marker bits they were written with, so the numbers here
//! are the numbers on the wire and the numbers in the specification.

/// The framing every file opens with, and the box everything else sits in.
pub(crate) const EBML_HEADER: u32 = 0x1A45_DFA3;
pub(crate) const SEGMENT: u32 = 0x1853_8067;

/// The index that says where in the segment each part begins.
pub(crate) const SEEK_HEAD: u32 = 0x114D_9B74;
pub(crate) const SEEK: u32 = 0x4DBB;
pub(crate) const SEEK_ID: u32 = 0x53AB;
pub(crate) const SEEK_POSITION: u32 = 0x53AC;

/// What the film says about itself, of which one field matters here.
pub(crate) const INFO: u32 = 0x1549_A966;
/// How many nanoseconds one unit of every timestamp in the file is.
pub(crate) const TIMESTAMP_SCALE: u32 = 0x002A_D7B1;

/// The files carried alongside the picture, of which the cover art is one.
pub(crate) const ATTACHMENTS: u32 = 0x1941_A469;
pub(crate) const ATTACHED_FILE: u32 = 0x61A7;
pub(crate) const FILE_NAME: u32 = 0x466E;
pub(crate) const FILE_MIME_TYPE: u32 = 0x4660;
pub(crate) const FILE_DATA: u32 = 0x465C;

/// The tracks, and what each of them says it is.
pub(crate) const TRACKS: u32 = 0x1654_AE6B;
pub(crate) const TRACK_ENTRY: u32 = 0xAE;
pub(crate) const TRACK_NUMBER: u32 = 0xD7;
pub(crate) const TRACK_TYPE: u32 = 0x83;
pub(crate) const CODEC_ID: u32 = 0x86;
pub(crate) const LANGUAGE: u32 = 0x0022_B59C;
pub(crate) const LANGUAGE_BCP47: u32 = 0x0022_B59D;
pub(crate) const FLAG_DEFAULT: u32 = 0x88;
pub(crate) const FLAG_FORCED: u32 = 0x55AA;
pub(crate) const FLAG_HEARING_IMPAIRED: u32 = 0x55AB;

/// The frames themselves, which everything a file says about itself comes
/// before. They are grouped into clusters that each carry a timestamp the
/// blocks inside them are counted from.
pub(crate) const CLUSTER: u32 = 0x1F43_B675;
pub(crate) const TIMESTAMP: u32 = 0xE7;
pub(crate) const SIMPLE_BLOCK: u32 = 0xA3;
pub(crate) const BLOCK_GROUP: u32 = 0xA0;
pub(crate) const BLOCK: u32 = 0xA1;
pub(crate) const BLOCK_DURATION: u32 = 0x9B;

/// The track type subtitles are declared under.
pub(crate) const SUBTITLE: u64 = 0x11;
