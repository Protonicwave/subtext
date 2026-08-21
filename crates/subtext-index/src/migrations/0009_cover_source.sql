-- Where each film's cover came from.
--
-- The path alone cannot say whether an image was picked by somebody or found
-- by a scan, and a scan that cannot tell the two apart has no way of leaving a
-- choice alone. The source says which, and it is what the next scan compares
-- when it finds a second candidate.
--
-- A library written by the previous release already answers this: a cover at
-- the film's own path is artwork inside the file, since that is the only thing
-- that was ever recorded that way, any other path is the picture beside it, and
-- no path at all is nothing found. Filling the column in from what is already
-- there is what keeps such a library looking exactly as it did, with no rescan
-- and nothing to redraw.
ALTER TABLE film ADD COLUMN cover_source TEXT NOT NULL DEFAULT 'none';

UPDATE film
SET cover_source = CASE
    WHEN cover_path IS NULL      THEN 'none'
    WHEN cover_path = film.path  THEN 'in-file'
    ELSE 'beside'
END;
