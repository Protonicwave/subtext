# Subtext

A desktop player for your film collection that treats subtitles as data rather than as pixels painted over the video. Every line of dialogue is parsed and indexed, which gives you a transcript panel that follows playback, and search across every line in your library. Nothing is uploaded and no files are copied or moved.

<!-- Demo to follow. -->

## Status

Early development. Not yet usable.

## Requirements

- [Rust](https://rustup.rs) 1.90 or later
- [Node.js](https://nodejs.org) 20 or later
- Platform prerequisites for Tauri, listed at [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites). On Windows this means the Visual Studio C++ build tools; on Linux, WebKitGTK and its development headers.

## Building

```sh
npm install
npm run tauri dev     # run in development
npm run tauri build   # produce a release build
```

## Checks

```sh
npm run lint          # ESLint
npm run format:check  # Prettier
npm run typecheck     # TypeScript
npm test              # front end tests
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

## Layout

| Path                   | Contains                                                  |
| ---------------------- | --------------------------------------------------------- |
| `crates/subtext-core`  | Domain types, subtitle parsing, filename pairing. No I/O. |
| `crates/subtext-index` | SQLite persistence, full text search, migrations.         |
| `crates/subtext-scan`  | Filesystem walking, watching, the ingest pipeline.        |
| `src-tauri`            | The application shell: commands, protocol, configuration. |
| `src`                  | React front end.                                          |

The crates do not depend on Tauri, so the parser and the index can be tested and benchmarked without launching the application.

## Codec support

Playback uses the system webview, so Subtext plays what the webview plays. H.264 video in an MP4 container works everywhere. HEVC and DTS audio often do not. Where a file cannot be decoded, Subtext says so rather than showing a black rectangle. Transcoding is out of scope.

## Licence

MIT. See [LICENSE](LICENSE).
