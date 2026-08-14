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

## Subtitles

Subtext reads subtitles from two places, and a film can have them in both.

**Files beside the film.** An SRT next to `Heat.1995.1080p.BluRay.mkv` is paired to it by name, with the resolution, source, codec, release group and language suffixes stripped from both before they are compared. Every file that matches is kept, not only the first one found.

**Tracks inside the film.** Matroska files, which is what `.mkv` and `.webm` are, often carry their subtitles in the container. Subtext reads the text tracks straight out of them, so a film with nothing beside it still gets a transcript, a dialogue scrubber and a place in your search results. Only the parts of the file that hold subtitles are read, nothing is extracted to a new file, and nothing is written back to your film.

MP4 files are not read this way. They can carry timed text, but do so rarely enough that a second reader for them is not worth writing. An MP4 needs an SRT beside it.

### Choosing between them

Where a film has more than one subtitle, press C for the list, which names each one by its language and says whether it is forced or for hearing impaired viewers. You can also turn subtitles off. Both the track you pick and turning them off are remembered for that film.

Until you pick one, the track is chosen by what it is rather than by the order it was found in: your preferred language from Settings first, then a full track ahead of a forced one, then an ordinary track ahead of one for hearing impaired viewers. Forced tracks caption signs and the occasional foreign line in a film you otherwise follow, and opening on one is the wrong answer often enough to be worth the rule.

### Tracks Subtext cannot use

Some subtitles in a Matroska file are pictures rather than text: PGS on a Blu-ray rip, VobSub on a DVD one. Subtext lists them and says plainly that it cannot read them. Turning pictures into text means optical character recognition, which is a large dependency and an accuracy figure the rest of the app does not have to apologise for. Where a film has only these, an SRT beside it is the way to get its dialogue in.

### When a subtitle does not match the film

A file that came from somewhere else says nothing about which encode it was timed against, so it can run early or late. While watching, `[` and `]` move it fifty milliseconds at a time and show the current offset over the picture, which is the fastest way to find the right value by ear. S opens the timing panel, which has the same nudges plus the framerate conversions that make a subtitle drift further out as the film goes on rather than being wrong by a fixed amount.

The correction is saved against that track and applied everywhere afterwards, including the transcript and the second a search result opens the film at. A track from inside the film needs none of this: it was timed against those exact frames by whoever made the file.

Two settings apply to every subtitle whatever it came from. A lead-in puts a line on screen slightly before it is spoken, and a minimum time on screen keeps a short line up long enough to be read. Both are what broadcast subtitling does and what a file timed to the first syllable does not, and both can be set to zero if you would rather have the file exactly as written.

### Working the offset out for you

Press A, or use the align action in the timing panel, and Subtext listens to the film. It reads the soundtrack, works out where the talking actually falls, and compares that with where the subtitle claims it does. Where the two agree plainly, the offset and any framerate conversion are set for you, and one press puts back whatever was in force before. A feature length film takes a few seconds. It carries on playing throughout, and the reading can be stopped at any point.

It is something you ask for rather than something done to you. Nothing is measured when films are added to the library, no film is read until you press the key, and a correction you arrived at yourself is never replaced without asking first.

Subtext decodes the audio itself for this, so the formats it can read are its own rather than the webview's. It reads AAC, MP3, FLAC, Vorbis, PCM and ALAC, which covers most of what a film is distributed in. It does not read AC-3, E-AC-3, DTS or TrueHD, which is what a disc rip usually carries. Those films say so by name and leave the bracket keys where they are. None of the audio is played, kept or written anywhere; it is read a packet at a time to be measured and thrown away.

Where the measurement is not good enough to act on, nothing is changed and it says so. A subtitle belonging to a different film, or a forced track of a few dozen lines with too little in it to line up, both end that way. Being told that nothing happened leaves you exactly where you were, with the keys above; a wrong answer applied quietly would leave you watching a film that is out from beginning to end, with no reason to suspect the file.

## Codec support

Playback uses the webview your system already has, so Subtext plays what the webview plays. H.264 video in an MP4 container works everywhere. HEVC video and DTS audio frequently do not, and support varies by platform.

Where a file cannot be decoded, Subtext says which part of it was refused rather than showing a black rectangle. Transcoding is out of scope: Subtext does not convert your files.

Lining a subtitle up by listening to the film is the one place Subtext decodes anything itself, and the boundary there is a different one, described [above](#working-the-offset-out-for-you). A film can play perfectly and still have a soundtrack that cannot be read for measurement, and the other way round.

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
cargo bench --workspace   # parser, index, scan and alignment benchmarks
```

CI runs all of these on Windows, macOS and Linux.

## Layout

| Path                       | Contains                                                  |
| -------------------------- | --------------------------------------------------------- |
| `crates/subtext-core`      | Domain types, subtitle parsing, filename pairing. No I/O. |
| `crates/subtext-container` | Reading the subtitle tracks inside a film file.           |
| `crates/subtext-index`     | SQLite persistence, full text search, migrations.         |
| `crates/subtext-scan`      | Filesystem walking, watching, the ingest pipeline.        |
| `crates/subtext-align`     | Lining two activity signals up. No files, no audio.       |
| `crates/subtext-speech`    | Decoding a film's audio to say where the talking is.      |
| `src-tauri`                | The application shell: commands, protocol, configuration. |
| `src`                      | React front end.                                          |

The crates do not depend on Tauri, so the parser and the index can be tested and benchmarked without launching the application.

## Not in version one

Transcoding. Downloading subtitles. Metadata from external databases. Reading subtitles that are pictures. Two subtitle tracks on screen at once, though you can keep several per film and switch between them. Casting. A light theme. Accounts or sync of any kind.

## Licence

MIT. See [LICENSE](LICENSE).
