"""Execute the shipped migration against real SQLite, without Rust tooling."""
import sqlite3
import unittest
from pathlib import Path
SCHEMA = (Path(__file__).parents[1] / 'src-tauri/migrations/001.sql').read_text()
class SchemaTests(unittest.TestCase):
 def setUp(self):
  self.db=sqlite3.connect(':memory:');self.db.executescript(SCHEMA);self.db.executescript((Path(__file__).parents[1] / 'src-tauri/migrations/002.sql').read_text());self.db.executescript((Path(__file__).parents[1] / 'src-tauri/migrations/003.sql').read_text())
 def tearDown(self): self.db.close()
 def test_migration_version(self):
  self.assertEqual(self.db.execute('pragma user_version').fetchone()[0],3)
 def test_unique_and_fts_delete(self):
  self.db.execute("insert into clipboard_items(kind,text,hash,first_copied,last_copied) values('text','Привет clipboard','hash',1,1)")
  self.assertEqual(self.db.execute("select count(*) from items_fts where items_fts match ?",('"Привет"*',)).fetchone()[0],1)
  with self.assertRaises(sqlite3.IntegrityError):
   self.db.execute("insert into clipboard_items(kind,text,hash,first_copied,last_copied) values('text','duplicate','hash',1,1)")
  self.db.execute('delete from clipboard_items')
  self.assertEqual(self.db.execute("select count(*) from items_fts where items_fts match 'clipboard'").fetchone()[0],0)
if __name__=='__main__':unittest.main()
