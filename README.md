# Subtext

<!--
  The demo belongs here, above everything else. Record it and add it as
  docs/demo.gif before tagging a release. It should open on the library:
  the shelves the folders made, one film opened for what its file is, then
  the subtitle lined up by listening to the film.
-->

A player for the films already on your disk.

Subtext reads the files themselves rather than asking anybody about them. That is the whole of it, and everything below follows from it:

- **Shelves made from your own folders.** However you sorted your films is how the library is arranged, because you already did the sorting and nobody else's taxonomy is going to beat it.
- **Covers taken off your disk.** The artwork inside the file, or the image beside it, or a frame from the film, in that order.
- **A page for every film that says what the file actually is.** Container, codec, resolution, bit depth, frame rate, every audio track, every subtitle track.
- **Subtitles chosen, corrected, and put right by listening to the film.** Press A and Subtext works out how far out they are, then shows you the lines landing on the voices so you do not have to take its word for it.

Nothing is uploaded and no network request is ever made. No file is copied, moved or written beside your films. Subtext watches folders that already exist on your disk, and a film that goes missing keeps its playback position until it comes back.

## Installing

Download the installer for your platform from [Releases](https://github.com/Protonicwave/subtext/releases), then point Subtext at a folder of films when it opens.

The macOS and Windows builds are not code signed, because that needs a paid developer certificate. Both will warn you the first time you open them: on macOS, right click the app and choose Open; on Windows, choose More info and then Run anyway.

## The library

One film sits at the top, the one you stopped part way through most recently, or the newest thing you added if you have not started anything. Play it, or open it for what it is.

Below that are rows, one for each folder inside a folder you asked Subtext to watch, named after the folder and in the order they were first found. Films sitting loose in a watched folder get a row under its name. There is nothing to tag, nothing to file and no genres to argue with: if your films are in folders called Westerns and Kurosawa, that is what the library says. Above the folders is a row of what you have not finished.

A wall of covers stops working somewhere around a couple of hundred films, so there is also a list, sortable by title, folder, year, runtime, video, size, when it was added and how much of it you have seen. Ctrl+L swaps between the two and Subtext remembers which you chose, along with how you last sorted it.

Ctrl+K finds a film by its title or by the folder it is filed in, without regard to case or to the accents over the letters, so `amelie` finds Amélie. The same list holds the things Subtext can do, each shown with its own key, so it teaches the shortcuts rather than standing in for them.

### What a film is

Opening a film shows its page before it plays. The top is the cover, the title, the year, how long it runs and where you stopped. Below that is what Subtext read out of the file: the container, the video codec, the resolution, the bit depth, the frame rate, the average bitrate, the size, when it was added and where it lives. Then every audio track with its language and codec, and every subtitle track with its language, whether it sits beside the film or inside it, and whether it is forced or for hearing impaired viewers.

A fact Subtext does not know is left out rather than shown empty. An MP4 is not taken apart the way a Matroska file is, so it says less, and saying less is the honest answer.

## Covers

A film's cover comes off your disk and from nowhere else. Subtext makes no network requests, so there is no artwork service to consent to and none to be wrong.

Three places are tried in order. An image attached inside a Matroska file, which is where a well made one keeps its artwork. An image beside the film, named after it or called `cover`, `poster` or `folder` in the film's own folder, which is the layout Plex, Jellyfin and Kodi have taught people to keep. Then a frame from the film itself, taken a fifth of the way in.

The first two were chosen by somebody and the third is a guess, which is why the guess comes last. A film with none of the three is drawn as a cover composed from its title, its year and how long it runs. Whatever is used, the image is cached in Subtext's own data directory and nothing is written beside your films.

## Subtitles

Subtext reads subtitles from two places, and a film can have them in both.

**Files beside the film.** An SRT next to `Heat.1995.1080p.BluRay.mkv` is paired to it by name, with the resolution, source, codec, release group and language suffixes stripped from both before they are compared. Every file that matches is kept, not only the first one found.

**Tracks inside the film.** Matroska files, which is what `.mkv` and `.webm` are, often carry their subtitles in the container. Subtext reads the text tracks straight out of them, so a film with nothing beside it still has its dialogue. Only the parts of the file that hold subtitles are read, nothing is extracted to a new file, and nothing is written back to your film.

MP4 files are not read this way. They can carry timed text, but do so rarely enough that a second reader for them is not worth writing. An MP4 needs an SRT beside it.

### Choosing between them

Where a film has more than one subtitle, press C for the list, which names each one by its language and says whether it is forced or for hearing impaired viewers. You can also turn subtitles off. Both the track you pick and turning them off are remembered for that film.

Until you pick one, the track is chosen by what it is rather than by the order it was found in: your preferred language from Settings first, then a full track ahead of a forced one, then an ordinary track ahead of one for hearing impaired viewers. Forced tracks caption signs and the occasional foreign line in a film you otherwise follow, and opening on one is the wrong answer often enough to be worth the rule.

### Tracks Subtext cannot use

Some subtitles in a Matroska file are pictures rather than text: PGS on a Blu-ray rip, VobSub on a DVD one. Subtext lists them and says plainly that it cannot read them. Turning pictures into text means optical character recognition, which is a large dependency and an accuracy figure the rest of the app does not have to apologise for. Where a film has only these, an SRT beside it is the way to get its dialogue in.

### When a subtitle does not match the film

A file that came from somewhere else says nothing about which encode it was timed against, so it can run early or late. While watching, `[` and `]` move it fifty milliseconds at a time and show the current offset over the picture, which is the fastest way to find the right value by ear. S opens the timing panel, which has the same nudges plus the framerate conversions that make a subtitle drift further out as the film goes on rather than being wrong by a fixed amount.

The correction is saved against that track and applied every time its dialogue is read afterwards. A track from inside the film needs none of this: it was timed against those exact frames by whoever made the file.

Two settings apply to every subtitle whatever it came from. A lead-in puts a line on screen slightly before it is spoken, and a minimum time on screen keeps a short line up long enough to be read. Both are what broadcast subtitling does and what a file timed to the first syllable does not, and both can be set to zero if you would rather have the file exactly as written.

Arrow keys land on the start of a line rather than an arbitrary ten seconds back, which is the one thing Subtext does with the dialogue beyond drawing it.

### Working the offset out for you

Press A, or use the align action in the timing panel or on a film's page, and Subtext works the offset out for you. It is something you ask for rather than something done to you: nothing is measured when films are added to the library, no film is read until you press the key, and a correction you arrived at yourself is never replaced without asking first. The film carries on playing throughout and the reading can be stopped at any point.

Where the film carries a text subtitle track of its own, that track is what the other subtitles are measured against. It was written for these exact frames, so it is timings against timings, the answer is good to a hundredth of a second, and no audio is read at all. It works whatever language either track is in, because dialogue falls at the same moments however it has been translated.

Otherwise Subtext listens to the film. It reads the soundtrack, works out where the talking actually falls, and compares that with where the subtitle claims it does. Lines that caption a sound rather than a voice, `[DOOR SLAMS]` or a lyric, are left out of the comparison, and they are still drawn on screen as they always were.

Subtext decodes that audio itself, so the formats it can read are its own rather than the webview's. It reads MP3, FLAC, Vorbis, PCM and ALAC, and AAC in the plain two channel form a download usually carries. It does not read AAC of more than two channels or in its high efficiency form, and it does not read AC-3, E-AC-3, DTS or TrueHD, which is what a disc rip usually carries. None of the audio is played, kept or written anywhere: it is read a packet at a time to be measured and thrown away. A film whose soundtrack Subtext cannot read can still be measured against a text subtitle track inside it, where it carries one, because that path reads no audio at all.

Either way the film is measured a stretch at a time, a couple of dozen stretches across its length, and the answer that wins is the one that explains all of them rather than the one that scores best in one place. That is what tells a subtitle that is out by a fixed amount from one that drifts wider as the film goes on because it was timed against a different framerate. A single measurement cannot tell those two apart, because a wrong stretch at a compensating offset looks very much like the right one.

#### It checks its own answer

A measurement you cannot check is worse than none. Nudging by ear has a floor: you can hear that you have made it worse and press the other key. A number applied to a film you have not watched yet has no floor at all, and if it is wrong it is wrong from beginning to end with nothing to suggest the tool rather than the file.

So Subtext counts. For the file as it stands, and again for the correction it has just measured, it counts how many lines arrive as somebody starts talking. Nothing is written unless that count goes up and clears a bar, and what you are told is both counts: seventy lines in a hundred landing on the talking, against twenty five before. Beside them is what the film said, which is how many of those stretches agreed and how closely.

No film puts every line on a voice. Whispers, lines off microphone and dialogue under a loud mix are speech a soundtrack reading will miss, so the figure is quoted next to the same figure for the file as it was rather than as a mark out of a hundred.

Then press V and watch it happen. Subtext goes to the busiest few seconds of dialogue in the whole film, plays them, and asks whether the lines arrive with the voices. Keep it, or put back whatever was in force before, which is one press and no confirmation because it is undoing something that has just happened. The film does not pause to ask.

#### What it refuses, and why refusing is the point

A correction that makes a film worse is not applied, whatever the arithmetic thought of it. Where that happens Subtext says what it measured, changes nothing, and leaves the bracket keys where they were.

It refuses by name, because which refusal it is tells you what to do next:

- **Not enough to go on.** A track of a few dozen lines, which is what a subtitle for signs and foreign dialogue looks like, has too little in it to tell one moment of a film from another.
- **Nothing to listen to**, for a film with no soundtrack, and **audio Subtext cannot read**, for one whose soundtrack is in a format there is no decoder for. The second names the format it found.
- **Not sure enough to say.** The lines do not fall where the talking does, whatever they are moved by. A subtitle belonging to another film ends here.
- **This film has breaks in it.** The subtitles fit up to a point, and after it everything is out by the same amount again, which is what an advertisement break cut into a recording does. One shift cannot put both parts right, so none is applied.
- **A subtitle for a different cut.** The subtitles fit up to a point, and then the film goes on talking through lines the subtitle does not have. It says roughly where that started. Stretching a theatrical subtitle onto an extended cut would look aligned and be wrong in every scene the two versions do not share, which is worse than leaving it alone.

Being told that nothing happened leaves you exactly where you were, with the keys above. A wrong answer applied quietly would leave you watching a film that is out from beginning to end, with no reason to suspect the file.

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
cargo bench --workspace   # parser, index, scan, container and alignment benchmarks
```

CI runs all of these on Windows, macOS and Linux.

## Layout

| Path                       | Contains                                                       |
| -------------------------- | -------------------------------------------------------------- |
| `crates/subtext-core`      | Domain types, subtitle parsing, filename pairing. No I/O.      |
| `crates/subtext-container` | Reading the tracks, the artwork and the details inside a film. |
| `crates/subtext-index`     | SQLite persistence, migrations, repositories.                  |
| `crates/subtext-scan`      | Filesystem walking, watching, the ingest pipeline.             |
| `crates/subtext-align`     | Lining two activity signals up. No files, no audio.            |
| `crates/subtext-speech`    | Decoding a film's audio to say where the talking is.           |
| `src-tauri`                | The application shell: commands, protocol, configuration.      |
| `src`                      | React front end.                                               |

The crates do not depend on Tauri, so the parser and the index can be tested and benchmarked without launching the application.

## Not here, on purpose

No metadata service, no artwork provider, no synopsis, no cast, no ratings. Subtext reads the disk, and adding a service would change what it claims about itself.

Also absent: transcoding, downloading subtitles, reading subtitles that are pictures, two subtitle tracks on screen at once, casting, a light theme, and accounts or sync of any kind. You can keep several subtitles per film and switch between them.

## Licence

MIT. See [LICENSE](LICENSE).
