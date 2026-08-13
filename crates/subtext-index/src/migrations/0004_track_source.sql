-- Where a subtitle track came from, and what it is written as.
--
-- Until now every track was a file sitting beside the film, so its path said
-- which track it was. A track inside the container carries the film's own path
-- and is told apart by the number the container knows it by, which means two
-- tracks of one film now share a path and the constraint that forbade it has to
-- go. It was declared on the column, and a constraint declared that way cannot
-- be dropped, so the table is built again and the rows are carried across.
--
-- Identifiers are carried across with them. A film pointing at the track it is
-- watched with, and a correction somebody arrived at by ear, both still point
-- at what they pointed at.
--
-- Foreign keys are off while migrations run. Dropping a table that other rows
-- reference would otherwise take the cues of every track in the library with
-- it, which is the whole library's worth of parsing to do again.
CREATE TABLE subtitle_track_next (
    id               INTEGER PRIMARY KEY,
    film_id          INTEGER NOT NULL REFERENCES film (id) ON DELETE CASCADE,
    -- The subtitle file, or the film itself for a track carried inside it.
    path             TEXT    NOT NULL,
    -- Whether the track is a file beside the film or a stream within it.
    origin           TEXT    NOT NULL,
    -- The number the container knows the track by, or zero for a file. Not a
    -- position in a list: a container numbers its tracks as it pleases, and the
    -- numbers need not start at one or run without gaps.
    stream_number    INTEGER NOT NULL,
    -- What the track is written as, which is what decides whether its dialogue
    -- can be read at all. A file beside the film is SubRip, since that is the
    -- only kind a scan picks up.
    codec            TEXT    NOT NULL,
    language         TEXT,
    forced           INTEGER NOT NULL,
    hearing_impaired INTEGER NOT NULL,
    match_kind       TEXT    NOT NULL,
    encoding         TEXT    NOT NULL,
    cue_count        INTEGER NOT NULL,
    size_bytes       INTEGER NOT NULL,
    modified_at      INTEGER NOT NULL,
    offset_ms        INTEGER NOT NULL DEFAULT 0,
    rate             REAL    NOT NULL DEFAULT 1.0
) STRICT;

INSERT INTO subtitle_track_next (
    id, film_id, path, origin, stream_number, codec, language, forced,
    hearing_impaired, match_kind, encoding, cue_count, size_bytes, modified_at,
    offset_ms, rate
)
SELECT id, film_id, path, 'sidecar', 0, 'subrip', language, forced,
       hearing_impaired, match_kind, encoding, cue_count, size_bytes,
       modified_at, offset_ms, rate
FROM subtitle_track;

DROP TABLE subtitle_track;
ALTER TABLE subtitle_track_next RENAME TO subtitle_track;

CREATE INDEX subtitle_track_by_film ON subtitle_track (film_id);

-- What identifies a track now that a path no longer does on its own.
CREATE UNIQUE INDEX subtitle_track_by_source
    ON subtitle_track (path, stream_number);

-- When a film was last looked inside for the tracks it carries.
--
-- Null for every film already in the table, which is what makes the first scan
-- after this look at all of them. Afterwards a film is opened again only when
-- its size or its modification time says it is not the file that was read, so a
-- rescan of a library nobody has touched opens nothing at all.
ALTER TABLE film ADD COLUMN probed_at INTEGER;
