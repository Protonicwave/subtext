//! Serving films to the webview, a range at a time.
//!
//! The video element is the platform's and the decoding is the platform's, but
//! the bytes are ours to hand over. A film is a file on disk that nothing may
//! read over HTTP, so it is served through a scheme of our own that answers
//! range requests properly. Without ranges the element would have to hold the
//! whole file before it could show a frame, and seeking a four gigabyte film
//! would mean reading four gigabytes.
//!
//! What may be served is decided by [`crate::allowed::Roots`], which holds the
//! folders somebody asked Subtext to watch and nothing else.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use tauri::http::{Method, Request, Response, StatusCode, Uri, header};
use tauri::{AppHandle, Manager, Runtime, UriSchemeContext, UriSchemeResponder};

use crate::allowed::Roots;

/// The scheme films are served under.
///
/// The front end turns a path into a URL for it with the same helper Tauri
/// gives the asset protocol, so this name appears in exactly two places: here,
/// and in the content security policy that permits it.
pub(crate) const SCHEME: &str = "stream";

/// How much of a file one response carries.
///
/// A player asking for `bytes=0-` means "the rest of it", and the rest of a
/// film is gigabytes that would be read into memory whole before a single frame
/// appeared. Answering with the first few megabytes instead is what every
/// streaming server does, and the element simply asks for the next range when
/// it wants more. Four megabytes is around twenty seconds of a 1080p encode:
/// long enough that requests are rare, short enough that a seek is one read.
const CHUNK: u64 = 4 * 1024 * 1024;

/// Answers a request for a film without holding up the window.
// The context is handed over by value because that is the shape Tauri asks a
// protocol handler to have, and only the handle inside it is wanted.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn handle<R: Runtime>(
    context: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = context.app_handle().clone();

    // Opening a file on a drive that has been asleep, and reading megabytes off
    // it, is not work for the thread the window is drawn on.
    tauri::async_runtime::spawn_blocking(move || {
        responder.respond(serve(&app, &request));
    });
}

fn serve<R: Runtime>(app: &AppHandle<R>, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    if request.method() != Method::GET {
        return refuse(StatusCode::METHOD_NOT_ALLOWED, "only GET is served here");
    }

    let Some(asked) = requested(request.uri()) else {
        return refuse(StatusCode::BAD_REQUEST, "that is not a path");
    };

    // The one check that matters. Everything below this line is reading a file
    // that a watched folder holds.
    let Some(path) = app.state::<Roots>().resolve(&asked) else {
        return refuse(
            StatusCode::FORBIDDEN,
            "that file is not in a watched folder",
        );
    };

    let Ok(file) = File::open(&path) else {
        return refuse(StatusCode::NOT_FOUND, "that file could not be opened");
    };
    let Ok(length) = file.metadata().map(|metadata| metadata.len()) else {
        return refuse(StatusCode::NOT_FOUND, "that file could not be measured");
    };

    let content_type = content_type_of(&path);

    // An empty file has no range to satisfy, and saying so as a 416 would leave
    // the element retrying rather than reporting that there is nothing here.
    if length == 0 {
        return match built(StatusCode::OK, content_type, Vec::new()) {
            Some(response) => response,
            None => refuse(StatusCode::INTERNAL_SERVER_ERROR, "that could not be sent"),
        };
    }

    let header = request
        .headers()
        .get(header::RANGE)
        .and_then(|range| range.to_str().ok());

    let Some((start, end)) = range_of(header, length) else {
        return unsatisfiable(length);
    };

    match read(&file, start, end) {
        Some(bytes) => partial(bytes, content_type, start, end, length),
        None => refuse(StatusCode::NOT_FOUND, "that file could not be read"),
    }
}

/// The path a request names.
///
/// The front end writes it the way Tauri's own asset protocol is addressed: the
/// whole path percent encoded as a single segment, so a film with a hash or a
/// question mark in its name arrives intact.
fn requested(uri: &Uri) -> Option<PathBuf> {
    let encoded = uri.path().strip_prefix('/')?;
    let decoded = percent_encoding::percent_decode_str(encoded)
        .decode_utf8()
        .ok()?;

    (!decoded.is_empty()).then(|| PathBuf::from(decoded.as_ref()))
}

/// The half open range a request asks for, as inclusive byte offsets.
///
/// Returns nothing when the request cannot be satisfied, which is a 416 and not
/// a guess: a player that asked for the wrong thing should be told so rather
/// than handed a different part of the film.
fn range_of(header: Option<&str>, length: u64) -> Option<(u64, u64)> {
    let Some(header) = header else {
        // No range asked for. Answering with the first chunk rather than the
        // whole file keeps a four gigabyte film out of memory, and a media
        // element treats it as the beginning of a stream it can seek within.
        return Some((0, chunked(0, length)));
    };

    let spec = header.trim().strip_prefix("bytes=")?;
    // Multiple ranges in one request are legal and no player sends them, so the
    // first is read and the rest ignored rather than answered as multipart.
    let spec = spec.split(',').next()?.trim();
    let (from, to) = spec.split_once('-')?;

    if from.is_empty() {
        // A suffix range: the last N bytes. Asking for more than there is means
        // the whole file rather than nothing.
        let wanted: u64 = to.parse().ok()?;
        if wanted == 0 {
            return None;
        }
        let start = length.saturating_sub(wanted);
        return Some((start, chunked(start, length)));
    }

    let start: u64 = from.parse().ok()?;
    if start >= length {
        return None;
    }

    let end = match to.trim() {
        "" => chunked(start, length),
        digits => {
            let asked: u64 = digits.parse().ok()?;
            if asked < start {
                return None;
            }
            asked.min(chunked(start, length))
        }
    };

    Some((start, end))
}

/// The last byte offset a response starting at `start` will carry.
fn chunked(start: u64, length: u64) -> u64 {
    start.saturating_add(CHUNK).min(length).saturating_sub(1)
}

/// The bytes between two inclusive offsets.
fn read(mut file: &File, start: u64, end: u64) -> Option<Vec<u8>> {
    let wanted = usize::try_from(end.saturating_sub(start).saturating_add(1)).ok()?;

    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = vec![0; wanted];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

/// What the webview plays, worked out from the name.
///
/// The container, never the codec. What is inside is the decoder's business,
/// and a file it cannot decode is reported by the player rather than guessed at
/// here.
fn content_type_of(path: &std::path::Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        _ => "application/octet-stream",
    }
}

fn partial(
    bytes: Vec<u8>,
    content_type: &'static str,
    start: u64,
    end: u64,
    length: u64,
) -> Response<Vec<u8>> {
    let range = format!("bytes {start}-{end}/{length}");
    let response = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_RANGE, range)
        .header(header::ACCEPT_RANGES, "bytes")
        // The films are served from a scheme of their own, which is a different
        // origin to the window. Without this the element will not play them,
        // and a frame taken from one would taint the canvas it is drawn on.
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(bytes);

    response.unwrap_or_else(|_| Response::new(Vec::new()))
}

fn built(
    status: StatusCode,
    content_type: &'static str,
    bytes: Vec<u8>,
) -> Option<Response<Vec<u8>>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(bytes)
        .ok()
}

/// A range that names bytes the file does not have.
fn unsatisfiable(length: u64) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::RANGE_NOT_SATISFIABLE)
        .header(header::CONTENT_RANGE, format!("bytes */{length}"))
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Vec::new())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

fn refuse(status: StatusCode, why: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(why.as_bytes().to_vec())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

#[cfg(test)]
mod tests {
    // A test that cannot write the file it is about to read has nothing to
    // measure, so it stops rather than passing quietly.
    #![allow(clippy::expect_used)]

    use std::path::Path;

    use super::{CHUNK, content_type_of, range_of, read, requested};

    /// A film of two chunks and a bit, so that the cap is visible.
    const LENGTH: u64 = CHUNK * 2 + 1_000;

    #[test]
    fn a_request_without_a_range_gets_the_beginning() {
        assert_eq!(range_of(None, LENGTH), Some((0, CHUNK - 1)));
        // A file smaller than one chunk comes back whole.
        assert_eq!(range_of(None, 500), Some((0, 499)));
    }

    #[test]
    fn an_open_ended_range_is_capped_at_a_chunk() {
        assert_eq!(range_of(Some("bytes=0-"), LENGTH), Some((0, CHUNK - 1)));
        assert_eq!(
            range_of(Some("bytes=1000-"), LENGTH),
            Some((1_000, 1_000 + CHUNK - 1))
        );
        // Near the end there is less than a chunk left.
        assert_eq!(
            range_of(Some("bytes=8388608-"), LENGTH),
            Some((CHUNK * 2, LENGTH - 1))
        );
    }

    #[test]
    fn a_closed_range_is_honoured_up_to_the_cap() {
        assert_eq!(range_of(Some("bytes=100-199"), LENGTH), Some((100, 199)));
        assert_eq!(range_of(Some("bytes=0-0"), LENGTH), Some((0, 0)));
        // Asking for more than a chunk gets a chunk, which is a shorter answer
        // and not a wrong one.
        assert_eq!(
            range_of(Some("bytes=0-99999999"), LENGTH),
            Some((0, CHUNK - 1))
        );
    }

    #[test]
    fn a_suffix_range_counts_back_from_the_end() {
        assert_eq!(
            range_of(Some("bytes=-500"), LENGTH),
            Some((LENGTH - 500, LENGTH - 1))
        );
        // More than the file holds is the whole file, not an error.
        assert_eq!(range_of(Some("bytes=-99999999999"), 400), Some((0, 399)));
    }

    #[test]
    fn a_range_past_the_end_is_refused() {
        assert_eq!(range_of(Some("bytes=400-"), 400), None);
        assert_eq!(range_of(Some("bytes=500-600"), 400), None);
        assert_eq!(range_of(Some("bytes=200-100"), 400), None);
        assert_eq!(range_of(Some("bytes=-0"), 400), None);
    }

    #[test]
    fn nonsense_is_refused_rather_than_guessed_at() {
        for rubbish in [
            "",
            "seconds=0-1",
            "bytes=",
            "bytes=abc-def",
            "bytes=1",
            "bytes=-abc",
        ] {
            assert_eq!(range_of(Some(rubbish), 400), None, "{rubbish}");
        }
    }

    #[test]
    fn only_the_first_of_several_ranges_is_answered() {
        assert_eq!(range_of(Some("bytes=0-99, 200-299"), LENGTH), Some((0, 99)));
    }

    #[test]
    fn a_path_comes_back_out_of_the_url_intact() {
        let uri = "stream://localhost/%2Ffilms%2FHeat%20%231%20%3F.mkv"
            .parse()
            .unwrap_or_default();
        assert_eq!(
            requested(&uri).as_deref(),
            Some(Path::new("/films/Heat #1 ?.mkv"))
        );

        let windows = "http://stream.localhost/C%3A%5Cfilms%5CHeat.mkv"
            .parse()
            .unwrap_or_default();
        assert_eq!(
            requested(&windows).as_deref(),
            Some(Path::new(r"C:\films\Heat.mkv"))
        );
    }

    #[test]
    fn a_url_naming_nothing_is_not_a_path() {
        let root = "stream://localhost/".parse().unwrap_or_default();
        assert_eq!(requested(&root), None);
    }

    /// The seek target, measured on the part of it that is ours.
    ///
    /// A seek costs one request, and a request costs one open, one seek and one
    /// read of a bounded size. What it does not cost is a read of the file,
    /// which is the failure this is here to catch: a change that dropped the
    /// cap would take a hundred and twenty eight megabytes to answer this and
    /// gigabytes to answer a real film.
    ///
    /// The file is large enough that the read is a real one against the disk
    /// rather than something the page cache had ready from a moment ago.
    #[test]
    fn a_seek_far_into_a_large_file_reads_one_chunk() {
        const SIZE: usize = 128 * 1024 * 1024;

        let directory = tempfile::tempdir().expect("somewhere to write a film");
        let path = directory.path().join("Heat.1995.mkv");

        std::fs::write(&path, vec![7; SIZE]).expect("a film to seek into");
        let file = std::fs::File::open(&path).expect("the film to be readable");

        let length = u64::try_from(SIZE).expect("a size that fits");
        // Two thirds of the way in, which is not where anything has been read.
        let start = length / 3 * 2;
        let (from, to) = range_of(Some(&format!("bytes={start}-")), length)
            .expect("a range that far in is satisfiable");

        let began = std::time::Instant::now();
        let bytes = read(&file, from, to).expect("the range to be readable");
        let took = began.elapsed();

        assert_eq!(
            bytes.len(),
            usize::try_from(CHUNK).expect("a chunk that fits")
        );
        assert!(
            took < std::time::Duration::from_millis(200),
            "a seek should not take {took:?}"
        );
    }

    #[test]
    fn a_container_is_named_by_its_extension() {
        assert_eq!(content_type_of(Path::new("/films/Heat.MP4")), "video/mp4");
        assert_eq!(
            content_type_of(Path::new("/films/Heat.mkv")),
            "video/x-matroska"
        );
        assert_eq!(
            content_type_of(Path::new("/films/Heat")),
            "application/octet-stream"
        );
    }
}
