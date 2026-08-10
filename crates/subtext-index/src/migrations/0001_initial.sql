-- The first schema.
--
-- Migrations run forwards only, so this file is fixed once it has shipped: a
-- database somewhere has already run it, and editing it would leave that
-- database and this file disagreeing with nothing to say so. Changes go in a
-- new numbered file.

-- A folder the library watches. Everything else hangs off one of these, so
-- removing a folder removes what was found inside it.
CREATE TABLE watched_folder (
    id       INTEGER PRIMARY KEY,
    path     TEXT    NOT NULL UNIQUE,
    added_at INTEGER NOT NULL
) STRICT;

-- A film is the file where it already is: the path is the identity, and
-- nothing is ever copied or moved. A file that disappears keeps its row and
-- gains a missing_since, so an unplugged drive does not throw away where
-- someone had got to.
CREATE TABLE film (
    id            INTEGER PRIMARY KEY,
    folder_id     INTEGER NOT NULL REFERENCES watched_folder (id) ON DELETE CASCADE,
    path          TEXT    NOT NULL UNIQUE,
    title         TEXT    NOT NULL,
    year          INTEGER,
    -- Size and modification time together are the fingerprint an incremental
    -- rescan compares against, so that an unchanged library costs one stat per
    -- file and no parsing at all.
    size_bytes    INTEGER NOT NULL,
    modified_at   INTEGER NOT NULL,
    duration_ms   INTEGER,
    poster_path   TEXT,
    -- The dominant colour pair taken from the poster, which drives the accent.
    accent        TEXT,
    missing_since INTEGER,
    added_at      INTEGER NOT NULL
) STRICT;

CREATE INDEX film_by_folder ON film (folder_id);

CREATE TABLE subtitle_track (
    id               INTEGER PRIMARY KEY,
    film_id          INTEGER NOT NULL REFERENCES film (id) ON DELETE CASCADE,
    path             TEXT    NOT NULL UNIQUE,
    language         TEXT,
    forced           INTEGER NOT NULL,
    hearing_impaired INTEGER NOT NULL,
    -- Whether the pairing was exact, approximate or made by hand. The import
    -- sheet shows it, because an approximate match is the one worth checking.
    match_kind       TEXT    NOT NULL,
    -- What the file turned out to be encoded as, shown when it needed guessing.
    encoding         TEXT    NOT NULL,
    cue_count        INTEGER NOT NULL,
    size_bytes       INTEGER NOT NULL,
    modified_at      INTEGER NOT NULL
) STRICT;

CREATE INDEX subtitle_track_by_film ON subtitle_track (film_id);

CREATE TABLE cue (
    id       INTEGER PRIMARY KEY,
    track_id INTEGER NOT NULL REFERENCES subtitle_track (id) ON DELETE CASCADE,
    -- Position in the track counted from one, as the parser assigned it rather
    -- than as the file numbered it.
    ordinal  INTEGER NOT NULL,
    start_ms INTEGER NOT NULL,
    end_ms   INTEGER NOT NULL,
    -- The alignment the file asked for, one to nine, or null for the default.
    position INTEGER,
    text     TEXT    NOT NULL
) STRICT;

-- Cues are read a whole track at a time in playback order, both to draw the
-- transcript and to find the line at a moment.
CREATE INDEX cue_by_track ON cue (track_id, start_ms);

-- External content, so a million lines of dialogue are stored once rather than
-- twice: the index holds the terms and points back at the cue table for the
-- text. The triggers below are the entire cost of that arrangement, and they
-- are what keeps the two in step.
--
-- Porter stemming so that searching for "running" finds "ran", and unicode61
-- so that accented dialogue tokenises like everything else.
-- The prefix indexes are what make the palette usable. Every keystroke is a
-- prefix search, and the first two or three characters of a word are the ones
-- that match the most terms: without an index for them, matching "the" as a
-- prefix means merging the lists for every word in the library beginning with
-- those letters, which takes tens of milliseconds. With them it is a lookup.
-- Longer prefixes match few enough terms to be quick either way.
CREATE VIRTUAL TABLE cue_search USING fts5 (
    text,
    content = 'cue',
    content_rowid = 'id',
    tokenize = 'porter unicode61',
    prefix = '2 3'
);

-- Whether the index is being kept in step cue by cue.
--
-- Adding one film to a library means a few thousand rows and the triggers are
-- the cheapest way to do it. Adding a thousand films at once means a million,
-- and feeding those through the index one at a time costs around ten times what
-- building the whole index in one pass does. So a bulk ingest sets this, the
-- triggers stand down, and the index is built once at the end.
--
-- It lives in the database rather than in memory so that a scan interrupted by
-- a power cut is still known about: the flag is found set on the next start,
-- and the index is rebuilt before anything is allowed to search it.
CREATE TABLE index_state (
    id   INTEGER PRIMARY KEY CHECK (id = 0),
    bulk INTEGER NOT NULL
) STRICT;

INSERT INTO index_state (id, bulk) VALUES (0, 0);

CREATE TRIGGER cue_indexed AFTER INSERT ON cue
WHEN (SELECT bulk FROM index_state WHERE id = 0) = 0
BEGIN
    INSERT INTO cue_search (rowid, text) VALUES (new.id, new.text);
END;

CREATE TRIGGER cue_unindexed AFTER DELETE ON cue
WHEN (SELECT bulk FROM index_state WHERE id = 0) = 0
BEGIN
    INSERT INTO cue_search (cue_search, rowid, text) VALUES ('delete', old.id, old.text);
END;

CREATE TRIGGER cue_reindexed AFTER UPDATE ON cue
WHEN (SELECT bulk FROM index_state WHERE id = 0) = 0
BEGIN
    INSERT INTO cue_search (cue_search, rowid, text) VALUES ('delete', old.id, old.text);
    INSERT INTO cue_search (rowid, text) VALUES (new.id, new.text);
END;

CREATE TABLE playback_position (
    film_id     INTEGER PRIMARY KEY REFERENCES film (id) ON DELETE CASCADE,
    position_ms INTEGER NOT NULL,
    -- What the player measured the film to be, which is not known until it has
    -- been opened once.
    duration_ms INTEGER,
    finished    INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
) STRICT;

-- Continue watching is the first row of the library and asks for this order
-- every time it is drawn.
CREATE INDEX playback_position_by_time ON playback_position (updated_at DESC);

CREATE TABLE preference (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;
