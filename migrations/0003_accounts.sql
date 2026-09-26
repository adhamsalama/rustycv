-- Accounts. Until now there was one implicit local user and nothing owned
-- anything; a CV and an application now belong to somebody.
--
-- `email` is stored already lowercased and trimmed by the server, so a plain
-- UNIQUE is enough to make "the same address" mean one account.
CREATE TABLE users (
    id            TEXT PRIMARY KEY NOT NULL,
    email         TEXT NOT NULL UNIQUE,
    -- An Argon2id PHC string: the parameters and salt travel with the hash, so
    -- raising the cost later does not invalidate the rows already written.
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL
);

-- Sessions live in the database rather than in a signed cookie so that signing
-- out actually revokes something. The token is stored as issued: anyone who can
-- read this table can already read every CV in the one next to it, so hashing
-- it would protect nothing that is not already lost.
CREATE TABLE sessions (
    token      TEXT PRIMARY KEY NOT NULL,
    user_id    TEXT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX idx_sessions_user ON sessions (user_id);

-- Nullable, and not because a CV may be ownerless from here on: rows written
-- before this migration have no owner to name. The first account created on
-- such a database adopts them, which is what `auth::signup` does.
ALTER TABLE cvs ADD COLUMN user_id TEXT REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE applications ADD COLUMN user_id TEXT REFERENCES users (id) ON DELETE CASCADE;

-- Every listing is now "this user's, in board or dashboard order".
CREATE INDEX idx_cvs_user ON cvs (user_id, updated_at DESC);
CREATE INDEX idx_applications_user ON applications (user_id, status, position);
