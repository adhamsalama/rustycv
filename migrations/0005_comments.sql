-- Comments left on a published CV by whoever holds its link. Anonymous by
-- design, so there is no user_id: the only owner is the CV's, through `cv_id`,
-- and deleting the CV takes its comments with it.
--
-- Keyed on the CV's `id` rather than its `public_id`: the link token is the
-- way in, not the identity, and the owner's own routes only know the `id`.
CREATE TABLE comments (
    id         TEXT PRIMARY KEY NOT NULL,
    cv_id      TEXT NOT NULL REFERENCES cvs (id) ON DELETE CASCADE,
    author     TEXT NOT NULL,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_comments_cv ON comments (cv_id, created_at);

-- The owner's switch. Off by default: an anonymous comment is shown to every
-- later visitor, so a CV only takes them once its owner has asked for that.
ALTER TABLE cvs ADD COLUMN comments_enabled INTEGER NOT NULL DEFAULT 0;
