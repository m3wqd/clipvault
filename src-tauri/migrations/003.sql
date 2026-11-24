BEGIN IMMEDIATE;
ALTER TABLE clipboard_items ADD COLUMN source TEXT;
PRAGMA user_version=3;
COMMIT;
