-- The job tracker. An application is *not* part of a CV document: nothing here
-- is ever rendered, so it stays out of `data` and lives in its own table.
--
-- `cv_id` records which CV was sent. ON DELETE SET NULL because deleting a CV
-- must not take the application history with it — the card outlives the CV it
-- was tailored from.
CREATE TABLE applications (
    id         TEXT    PRIMARY KEY NOT NULL,
    company    TEXT    NOT NULL,
    role       TEXT    NOT NULL,
    url        TEXT    NOT NULL DEFAULT '',
    notes      TEXT    NOT NULL DEFAULT '',
    status     TEXT    NOT NULL
               CHECK (status IN ('wishlist', 'applied', 'interview', 'offer', 'rejected')),
    -- Order within a column. Per-status and contiguous only where a drag has
    -- rewritten it; gaps are harmless because nothing reads it but ORDER BY.
    position   INTEGER NOT NULL DEFAULT 0,
    cv_id      TEXT    REFERENCES cvs (id) ON DELETE SET NULL,
    created_at TEXT    NOT NULL,
    updated_at TEXT    NOT NULL
);

-- The board reads the whole table in board order, every time.
CREATE INDEX idx_applications_board ON applications (status, position);
