-- A CV is stored as its JSON document, never as a rendered PDF.
-- The PDF is always recomputed from `data` + the template it names.
CREATE TABLE cvs (
    id             TEXT    PRIMARY KEY NOT NULL,
    title          TEXT    NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    data           TEXT    NOT NULL,
    created_at     TEXT    NOT NULL,
    updated_at     TEXT    NOT NULL
);

-- The dashboard lists most-recently-edited first.
CREATE INDEX idx_cvs_updated_at ON cvs (updated_at DESC);
