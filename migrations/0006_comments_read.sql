-- When the owner last opened a CV's comments, as the `created_at` of the newest
-- comment they had been shown. Anything newer is unread. NULL means never
-- opened, so every comment counts.
ALTER TABLE cvs ADD COLUMN comments_read_at TEXT;
