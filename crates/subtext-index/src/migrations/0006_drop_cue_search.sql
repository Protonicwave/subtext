-- Take away the full text mirror over cue text.
--
-- Searching every line of dialogue in the library has been withdrawn, and the
-- index that served it is the only thing here that was ever for it. Dropping
-- the virtual table drops its shadow tables with it, and the triggers that kept
-- it in step go with the thing they were keeping in step.
--
-- The cues themselves stay. They are what makes opening a film instant rather
-- than a parse away, and they are what the alignment engine reads when it
-- compares a subtitle's authored timings against the speech in the film.
DROP TRIGGER IF EXISTS cue_indexed;
DROP TRIGGER IF EXISTS cue_unindexed;
DROP TRIGGER IF EXISTS cue_reindexed;
DROP TABLE IF EXISTS cue_search;

-- Whether the mirror was being kept in step row by row, which was a question
-- only a bulk ingest ever asked.
DROP TABLE IF EXISTS index_state;

-- The last few things somebody searched for, kept as a preference because they
-- were one short list rather than a table's worth. Nothing reads them now.
DELETE FROM preference WHERE key = 'search.recent';
