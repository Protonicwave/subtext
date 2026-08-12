-- Where a subtitle track's timings sit against the film it is paired with.
--
-- Two columns because they answer two different questions. An offset is a file
-- timed against a release whose intro was a different length; a rate is a file
-- timed against a release at a different framerate, which drifts further out
-- the longer the film runs. A file can need one, the other, or neither.
--
-- Both default to saying nothing, so every row already in the table reads back
-- exactly as it did before this ran, and a library nobody corrects never leaves
-- the identity case.
ALTER TABLE subtitle_track ADD COLUMN offset_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE subtitle_track ADD COLUMN rate REAL NOT NULL DEFAULT 1.0;
