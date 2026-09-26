-- A CV can be shared over a public link. `public_id` is the link's token —
-- separate from `id` so the internal, ever-incrementing primary key is never
-- itself the thing handed out in a URL. It is assigned once and kept across
-- unpublish/republish, so toggling the switch does not break a link someone
-- already has; `published` is what actually gates whether it answers.
-- No inline UNIQUE: SQLite's ALTER TABLE cannot add one directly. The
-- uniqueness lives in the partial index below instead.
ALTER TABLE cvs ADD COLUMN public_id TEXT;
ALTER TABLE cvs ADD COLUMN published INTEGER NOT NULL DEFAULT 0;

-- The rendered PDF for the published link, memoized rather than recomputed on
-- every visit. It is still a pure function of the document — `public_pdf` is
-- only ever read back alongside `public_pdf_cached_for`, and is thrown away
-- and rebuilt the moment that no longer matches `updated_at`. Never the
-- source of truth, only a cache of it, and only kept for CVs that are
-- actually published.
ALTER TABLE cvs ADD COLUMN public_pdf BLOB;
ALTER TABLE cvs ADD COLUMN public_pdf_cached_for TEXT;

CREATE UNIQUE INDEX idx_cvs_public_id ON cvs (public_id) WHERE public_id IS NOT NULL;
