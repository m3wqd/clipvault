PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS clipboard_items (
 id INTEGER PRIMARY KEY, kind TEXT NOT NULL, text TEXT NOT NULL,
 hash TEXT NOT NULL UNIQUE, first_copied INTEGER NOT NULL,
 last_copied INTEGER NOT NULL, copy_count INTEGER NOT NULL DEFAULT 1,
 pinned INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS items_recent ON clipboard_items(last_copied DESC);
CREATE INDEX IF NOT EXISTS items_pinned ON clipboard_items(pinned, last_copied DESC);
CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(text, content='clipboard_items', content_rowid='id', tokenize='unicode61');
CREATE TRIGGER IF NOT EXISTS items_ai AFTER INSERT ON clipboard_items BEGIN
 INSERT INTO items_fts(rowid,text) VALUES(new.id,new.text); END;
CREATE TRIGGER IF NOT EXISTS items_ad AFTER DELETE ON clipboard_items BEGIN
 INSERT INTO items_fts(items_fts,rowid,text) VALUES('delete',old.id,old.text); END;
PRAGMA user_version=1;
