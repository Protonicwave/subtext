-- Where a film's cover comes from.
--
-- The film's own path where the artwork is attached inside the container, the
-- picture beside it where somebody put one there, and nothing at all where
-- neither is true and a frame taken from the film is the only answer.
--
-- Recorded rather than worked out each time so that a film keeps the cover
-- somebody chose. Without it, a frame captured later would quietly replace an
-- image that was put there on purpose.
ALTER TABLE film ADD COLUMN cover_path TEXT;
