-- What each film's file turned out to be.
--
-- Until now a film was a path, a title, a year and a size. The screens being
-- built on top of this describe the file itself, and every fact they show is
-- read once when the film is opened and held here, so that opening a sheet
-- costs no disk at all.
--
-- Every column is nullable, and that is the point. A fact the file did not
-- state reads as not known rather than as zero, and an MP4, which this
-- application does not parse, carries only what the filesystem and the
-- container name can supply.
ALTER TABLE film ADD COLUMN container TEXT;
ALTER TABLE film ADD COLUMN video_codec TEXT;
ALTER TABLE film ADD COLUMN video_width INTEGER;
ALTER TABLE film ADD COLUMN video_height INTEGER;
ALTER TABLE film ADD COLUMN bit_depth INTEGER;
ALTER TABLE film ADD COLUMN frame_rate REAL;

-- The sound tracks a film carries.
--
-- A table rather than a column, because a film carries several of them and each
-- is a record rather than a value. It is the shape the subtitle tracks already
-- take, for the same reason, and it means a film losing a track when it is
-- re-encoded loses a row rather than leaving a string to be re-cut.
--
-- Nothing is decoded from these. They are named on a screen and that is all,
-- which is why the codec is kept as the identifier the file wrote rather than
-- as a name this build happens to know.
CREATE TABLE audio_stream (
    id            INTEGER PRIMARY KEY,
    film_id       INTEGER NOT NULL REFERENCES film (id) ON DELETE CASCADE,
    -- The number the container knows the track by. Not a position in a list:
    -- numbers are chosen by whoever wrote the file.
    stream_number INTEGER NOT NULL,
    codec         TEXT    NOT NULL,
    -- How many channels, from which the layout is named. Null where the file
    -- did not say.
    channels      INTEGER,
    language      TEXT,
    -- The track the film suggests, which is the one that will be heard.
    is_default    INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX audio_stream_by_film ON audio_stream (film_id, stream_number);
