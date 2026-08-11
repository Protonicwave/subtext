# Subtext

<!--
  The demo belongs here, above everything else: Ctrl+K, a line of dialogue
  typed, and the film opening at that moment. Record it and add it as
  docs/demo.gif before tagging a release.
-->

A desktop player for your film collection that treats subtitles as data rather than as pixels painted over the video.

Most players draw a subtitle file over the picture and forget it. Subtext parses every line once and indexes it, which gives you three things that follow from that one decision:

- **A transcript beside the film** that follows playback. Click any line to seek to it. Select a passage and copy it with its timecodes.
- **Search across every line of dialogue in your library.** Ctrl+K, type a half remembered line, and open the film at the second it is spoken.
- **A scrubber that shows where the dialogue is**, so quiet stretches read as valleys, and arrow keys that land on the start of a line rather than an arbitrary ten seconds back.

Nothing is uploaded. No file is copied, moved or written beside your films. Subtext watches folders that already exist on your disk, and a film that goes missing keeps its playback position until it comes back.

## Installing

Download the installer for your platform from [Releases](https://github.com/Protonicwave/subtext/releases), then point Subtext at a folder of films when it opens.

The macOS and Windows builds are not code signed, because that needs a paid developer certificate. Both will warn you the first time you open them: on macOS, right click the app and choose Open; on Windows, choose More info and then Run anyway.

## Codec support

Playback uses the webview your system already has, so Subtext plays what the webview plays. H.264 video in an MP4 container works everywhere. HEVC video and DTS audio frequently do not, and support varies by platform.

Where a file cannot be decoded, Subtext says which part of it was refused rather than showing a black rectangle. Transcoding is out of scope: Subtext does not ship a decoder and does not convert your files.

Subtitles are read from SRT files sitting beside the film, paired by name.

## Building

You need [Rust](https://rustup.rs) 1.90 or later, [Node.js](https://nodejs.org) 20 or later, and the platform prerequisites listed at [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites). On Windows that means the Visual Studio C++ build tools; on Linux, WebKitGTK and its development headers.

```sh
npm install
npm run tauri dev     # run in development
npm run tauri build   # produce installers in src-tauri/target/release/bundle
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
cargo bench --workspace   # parser, index and scan benchmarks
```

CI runs all of these on Windows, macOS and Linux.

## Layout

| Path                   | Contains                                                  |
| ---------------------- | --------------------------------------------------------- |
| `crates/subtext-core`  | Domain types, subtitle parsing, filename pairing. No I/O. |
| `crates/subtext-index` | SQLite persistence, full text search, migrations.         |
| `crates/subtext-scan`  | Filesystem walking, watching, the ingest pipeline.        |
| `src-tauri`            | The application shell: commands, protocol, configuration. |
| `src`                  | React front end.                                          |

The crates do not depend on Tauri, so the parser and the index can be tested and benchmarked without launching the application.

## Not in version one

Transcoding. Downloading subtitles. Metadata from external databases. Two subtitle tracks at once. Casting. A light theme. Accounts or sync of any kind.

## Licence

MIT. See [LICENSE](LICENSE).
