-- Which subtitle track a film is watched with.
--
-- Null for every film already in the table and for every film nobody has
-- chosen for, which is most of them. The track is then picked by a rule from
-- what the pairing found, so a library that has never been touched behaves the
-- way it always did.
--
-- Turning subtitles off is a choice in its own right and not the same as never
-- having made one: the first says show nothing, the second says work it out.
-- One nullable identifier cannot say three things, so there are two columns.
--
-- The reference clears itself when the track goes, which is what a subtitle
-- file being deleted or attached to a different film comes to. The film then
-- chooses again rather than pointing at a row that is not there.
ALTER TABLE film ADD COLUMN chosen_track_id INTEGER
    REFERENCES subtitle_track (id) ON DELETE SET NULL;
ALTER TABLE film ADD COLUMN subtitles_off INTEGER NOT NULL DEFAULT 0;
