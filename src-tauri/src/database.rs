use crate::error::{Error, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
pub const MAX_BYTES: usize = 1024 * 1024;
#[derive(Debug, Serialize)]
pub struct Item {
    pub id: i64,
    pub kind: String,
    pub text: String,
    pub first_copied: i64,
    pub last_copied: i64,
    pub copy_count: i64,
    pub pinned: bool,
    pub favorite: bool,
    pub image_path: Option<String>,
    pub source: Option<String>,
}
#[derive(Default)]
pub struct Search<'a> {
    pub query: &'a str,
    pub pinned: bool,
    pub kind: &'a str,
    pub favorite: bool,
    pub collection: Option<i64>,
    pub tag: &'a str,
    pub since: i64,
    pub frequent: bool,
    pub offset: u32,
}
#[cfg(test)]
impl<'a> Search<'a> {
    pub fn new(query: &'a str, pinned: bool, kind: &'a str) -> Self {
        Self {
            query,
            pinned,
            kind,
            ..Self::default()
        }
    }
}
pub struct Database(Connection);
#[derive(serde::Serialize)]
pub struct Snippet {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub category: String,
}
#[derive(serde::Serialize)]
pub struct Named {
    pub id: i64,
    pub name: String,
}
impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let c = Connection::open(path)?;
        c.busy_timeout(std::time::Duration::from_secs(3))?;
        let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 3 {
            return Err(Error::Invalid(
                "Database belongs to a newer ClipVault version".into(),
            ));
        }
        if version == 0 {
            c.execute_batch(include_str!("../migrations/001.sql"))?;
        }
        if version < 2 {
            c.execute_batch(include_str!("../migrations/002.sql"))?;
        }
        if version < 3 {
            c.execute_batch(include_str!("../migrations/003.sql"))?;
        }
        c.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(Self(c))
    }
    pub fn settings(&self) -> Result<crate::settings::Settings> {
        let value = self.0.query_row(
            "SELECT value FROM settings WHERE key='preferences'",
            [],
            |r| r.get::<_, String>(0),
        );
        match value {
            Ok(v) => serde_json::from_str(&v)
                .map_err(|_| Error::Invalid("Invalid stored settings".into())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(crate::settings::Settings::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save_settings(&self, s: &crate::settings::Settings) -> Result<()> {
        s.validate()?;
        let value = serde_json::to_string(s)
            .map_err(|_| Error::Invalid("Cannot serialize settings".into()))?;
        self.0.execute("INSERT INTO settings(key,value) VALUES('preferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[value])?;
        Ok(())
    }
    pub fn add(&mut self, text: &str) -> Result<bool> {
        if text.trim().is_empty() {
            return Ok(false);
        }
        if text.len() > self.settings()?.max_bytes {
            return Err(Error::Invalid("Clipboard text exceeds 1 MiB".into()));
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Invalid("System clock precedes Unix epoch".into()))?
            .as_millis() as i64;
        let kind = if text.starts_with("https://") || text.starts_with("http://") {
            "url"
        } else if text.contains('\n')
            && (text.contains("fn ") || text.contains("const ") || text.contains("def "))
        {
            "code"
        } else {
            "text"
        };
        let tx = self.0.transaction()?;
        tx.execute("INSERT INTO clipboard_items(kind,text,hash,first_copied,last_copied) VALUES(?1,?2,?3,?4,?4) ON CONFLICT(hash) DO UPDATE SET last_copied=excluded.last_copied,copy_count=copy_count+1",params![kind,text,blake3::hash(text.as_bytes()).to_hex().as_str(),now])?;
        let settings_value: Option<String> = tx
            .query_row(
                "SELECT value FROM settings WHERE key='preferences'",
                [],
                |r| r.get(0),
            )
            .ok();
        let settings = settings_value
            .and_then(|s| serde_json::from_str::<crate::settings::Settings>(&s).ok())
            .unwrap_or_default();
        tx.execute("DELETE FROM clipboard_items WHERE pinned=0 AND (id NOT IN (SELECT id FROM clipboard_items WHERE pinned=0 ORDER BY last_copied DESC,id DESC LIMIT ?1) OR (?2>0 AND last_copied<?3))",params![settings.max_items,settings.retention_days,now-(settings.retention_days as i64)*86400000])?;
        tx.commit()?;
        Ok(true)
    }
    pub fn list(&self, search: &Search<'_>) -> Result<Vec<Item>> {
        let Search {
            query,
            pinned,
            kind,
            favorite,
            collection,
            tag,
            since,
            frequent,
            offset,
        } = *search;
        if query.len() > 512 || !["all", "text", "url", "code", "image"].contains(&kind) {
            return Err(Error::Invalid("Invalid search parameters".into()));
        }
        let fts = query
            .split_whitespace()
            .map(|s| format!("\"{}\"*", s.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let mut st=self.0.prepare("SELECT id,kind,text,first_copied,last_copied,copy_count,pinned,favorite,image_path,source FROM clipboard_items WHERE (?1=0 OR pinned=1) AND (?2='all' OR kind=?2) AND (?3='' OR id IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?3)) AND (?4=0 OR favorite=1) AND (?5 IS NULL OR id IN (SELECT item_id FROM collection_items WHERE collection_id=?5)) AND (?6='' OR id IN (SELECT item_id FROM item_tags JOIN tags ON tags.id=item_tags.tag_id WHERE name=?6)) AND last_copied>=?7 ORDER BY pinned DESC,CASE WHEN ?8 THEN copy_count ELSE last_copied END DESC,id DESC LIMIT 200 OFFSET ?9")?;
        let rows = st.query_map(
            params![pinned, kind, fts, favorite, collection, tag, since, frequent, offset],
            |r| {
                Ok(Item {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    text: r.get(2)?,
                    first_copied: r.get(3)?,
                    last_copied: r.get(4)?,
                    copy_count: r.get(5)?,
                    pinned: r.get(6)?,
                    favorite: r.get(7)?,
                    image_path: r.get(8)?,
                    source: r.get(9)?,
                })
            },
        )?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn text(&self, id: i64) -> Result<String> {
        Ok(self
            .0
            .query_row("SELECT text FROM clipboard_items WHERE id=?1", [id], |r| {
                r.get(0)
            })?)
    }
    pub fn pin(&self, id: i64, value: bool) -> Result<()> {
        self.0.execute(
            "UPDATE clipboard_items SET pinned=?1 WHERE id=?2",
            params![value, id],
        )?;
        Ok(())
    }
    pub fn delete(&self, id: i64) -> Result<()> {
        self.0
            .execute("DELETE FROM clipboard_items WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn record_source(&self, hash: &str, source: Option<&str>) -> Result<()> {
        self.0.execute(
            "UPDATE clipboard_items SET source=?1 WHERE hash=?2",
            params![source, hash],
        )?;
        Ok(())
    }
    pub fn rename_collection(&self, id: i64, name: &str) -> Result<()> {
        if name.trim().is_empty() || name.len() > 100 {
            return Err(Error::Invalid("Invalid collection name".into()));
        }
        self.0.execute(
            "UPDATE collections SET name=?1 WHERE id=?2",
            params![name, id],
        )?;
        Ok(())
    }
    pub fn delete_collection(&self, id: i64) -> Result<()> {
        self.0
            .execute("DELETE FROM collections WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn favorite(&self, id: i64, value: bool) -> Result<()> {
        self.0.execute(
            "UPDATE clipboard_items SET favorite=?1 WHERE id=?2",
            params![value, id],
        )?;
        Ok(())
    }
    pub fn snippets(&self, query: &str) -> Result<Vec<Snippet>> {
        let mut st=self.0.prepare("SELECT id,title,body,category FROM snippets WHERE instr(lower(title || ' ' || body || ' ' || category),lower(?1))>0 ORDER BY title LIMIT 500")?;
        let rows = st.query_map([query], |r| {
            Ok(Snippet {
                id: r.get(0)?,
                title: r.get(1)?,
                body: r.get(2)?,
                category: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn save_snippet(
        &self,
        id: Option<i64>,
        title: &str,
        body: &str,
        category: &str,
    ) -> Result<()> {
        if title.trim().is_empty()
            || title.len() > 200
            || body.len() > MAX_BYTES
            || category.len() > 100
        {
            return Err(Error::Invalid("Invalid snippet".into()));
        }
        self.0.execute("INSERT INTO snippets(id,title,body,category) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET title=excluded.title,body=excluded.body,category=excluded.category",params![id,title,body,category])?;
        Ok(())
    }
    pub fn delete_snippet(&self, id: i64) -> Result<()> {
        self.0.execute("DELETE FROM snippets WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn collections(&self) -> Result<Vec<Named>> {
        let mut st = self
            .0
            .prepare("SELECT id,name FROM collections ORDER BY name")?;
        let rows = st.query_map([], |r| {
            Ok(Named {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn collection(&self, name: &str) -> Result<()> {
        if name.trim().is_empty() || name.len() > 100 {
            return Err(Error::Invalid(
                "Collection name must contain 1–100 bytes".into(),
            ));
        }
        self.0
            .execute("INSERT INTO collections(name) VALUES(?1)", [name])?;
        Ok(())
    }
    pub fn assign(&self, id: i64, collection: i64, value: bool) -> Result<()> {
        if value {
            self.0.execute(
                "INSERT OR IGNORE INTO collection_items VALUES(?1,?2)",
                params![id, collection],
            )?;
        } else {
            self.0.execute(
                "DELETE FROM collection_items WHERE item_id=?1 AND collection_id=?2",
                params![id, collection],
            )?;
        }
        Ok(())
    }
    pub fn tags(&mut self, id: i64, names: Vec<String>) -> Result<()> {
        if names.len() > 20 || names.iter().any(|n| n.is_empty() || n.len() > 100) {
            return Err(Error::Invalid("Invalid tags".into()));
        }
        let tx = self.0.transaction()?;
        tx.execute("DELETE FROM item_tags WHERE item_id=?1", [id])?;
        for name in names {
            tx.execute("INSERT OR IGNORE INTO tags(name) VALUES(?1)", [&name])?;
            tx.execute(
                "INSERT OR IGNORE INTO item_tags SELECT ?1,id FROM tags WHERE name=?2",
                params![id, name],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn get_tags(&self, id: i64) -> Result<Vec<String>> {
        let mut st=self.0.prepare("SELECT name FROM tags JOIN item_tags ON tags.id=item_tags.tag_id WHERE item_id=?1 ORDER BY name")?;
        let rows = st.query_map([id], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn clear(&self, all: bool) -> Result<()> {
        self.0
            .execute("DELETE FROM clipboard_items WHERE ?1 OR pinned=0", [all])?;
        Ok(())
    }
    pub fn add_image(&mut self, path: &str, hash: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Invalid("Invalid clock".into()))?
            .as_millis() as i64;
        self.0.execute("INSERT INTO clipboard_items(kind,text,hash,first_copied,last_copied,image_path) VALUES('image','Image',?1,?2,?2,?3) ON CONFLICT(hash) DO UPDATE SET last_copied=excluded.last_copied,copy_count=copy_count+1",params![hash,now,path])?;
        self.prune()?;
        Ok(())
    }
    pub fn image_path(&self, id: i64) -> Result<Option<String>> {
        Ok(self.0.query_row(
            "SELECT image_path FROM clipboard_items WHERE id=?1",
            [id],
            |r| r.get(0),
        )?)
    }
    pub fn prune(&self) -> Result<()> {
        let s = self.settings()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Invalid("Invalid system clock".into()))?
            .as_millis() as i64;
        self.0.execute("DELETE FROM clipboard_items WHERE pinned=0 AND (id NOT IN (SELECT id FROM clipboard_items WHERE pinned=0 ORDER BY last_copied DESC,id DESC LIMIT ?1) OR (?2>0 AND last_copied<?3))",params![s.max_items,s.retention_days,now-s.retention_days as i64*86400000])?;
        Ok(())
    }
    pub fn image_paths(&self) -> Result<Vec<String>> {
        let mut st = self
            .0
            .prepare("SELECT image_path FROM clipboard_items WHERE image_path IS NOT NULL")?;
        let rows = st.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Database {
        Database::open(Path::new(":memory:")).expect("memory database")
    }
    #[test]
    fn persistence_dedup_search_pin_delete() {
        let mut d = db();
        d.add("hello world").unwrap();
        d.add("hello world").unwrap();
        let items = d.list(&Search::new("hello", false, "all")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].copy_count, 2);
        let id = items[0].id;
        d.pin(id, true).unwrap();
        assert_eq!(d.list(&Search::new("", true, "all")).unwrap().len(), 1);
        d.delete(id).unwrap();
        assert!(d
            .list(&Search::new("hello", false, "all"))
            .unwrap()
            .is_empty());
    }
    #[test]
    fn survives_reopen() {
        let path = std::env::temp_dir().join(format!(
            "clipvault-test-{}-{}.db",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let mut d = Database::open(&path).unwrap();
            d.add("persistent Unicode: Привет").unwrap();
        }
        {
            let d = Database::open(&path).unwrap();
            assert_eq!(
                d.list(&Search::new("Привет", false, "all")).unwrap().len(),
                1
            );
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn empty_oversize_and_invalid() {
        let mut d = db();
        assert!(!d.add(" ").unwrap());
        assert!(d.add(&"x".repeat(MAX_BYTES + 1)).is_err());
        assert!(d.list(&Search::new("", false, "bad")).is_err());
        assert!(d.text(-1).is_err());
    }
    #[test]
    fn retention_preserves_pinned() {
        let mut d = db();
        d.add("keep").unwrap();
        let id = d.list(&Search::new("", false, "all")).unwrap()[0].id;
        d.pin(id, true).unwrap();
        for i in 0..1010 {
            d.add(&format!("entry {i}")).unwrap();
        }
        assert_eq!(d.text(id).unwrap(), "keep");
        let n: i64 =
            d.0.query_row("SELECT count(*) FROM clipboard_items", [], |r| r.get(0))
                .unwrap();
        assert_eq!(n, 1001);
    }
    #[test]
    fn favorites_tags_collections_and_cascade() {
        let mut d = db();
        d.add("tagged entry").unwrap();
        let id = d.list(&Search::new("", false, "all")).unwrap()[0].id;
        d.favorite(id, true).unwrap();
        d.collection("Work").unwrap();
        let c = d.collections().unwrap()[0].id;
        d.assign(id, c, true).unwrap();
        d.tags(id, vec!["rust".into(), "code".into()]).unwrap();
        let search = Search {
            favorite: true,
            collection: Some(c),
            tag: "rust",
            ..Search::new("", false, "all")
        };
        assert_eq!(d.list(&search).unwrap().len(), 1);
        d.delete(id).unwrap();
        assert!(d.list(&search).unwrap().is_empty());
        assert!(d.get_tags(id).unwrap().is_empty());
        let n: i64 =
            d.0.query_row("SELECT count(*) FROM collection_items", [], |r| r.get(0))
                .unwrap();
        assert_eq!(n, 0);
    }
    #[test]
    fn settings_snippets_and_clear() {
        let mut d = db();
        let settings = crate::settings::Settings {
            max_items: 10,
            ..Default::default()
        };
        d.save_settings(&settings).unwrap();
        assert_eq!(d.settings().unwrap().max_items, 10);
        d.save_snippet(None, "Reply", "Hello there", "Work")
            .unwrap();
        let sn = d.snippets("Hello").unwrap();
        assert_eq!(sn.len(), 1);
        d.save_snippet(Some(sn[0].id), "Updated", "Bye", "Work")
            .unwrap();
        assert!(d.snippets("Hello").unwrap().is_empty());
        d.delete_snippet(sn[0].id).unwrap();
        d.add("keep").unwrap();
        let id = d.list(&Search::new("", false, "all")).unwrap()[0].id;
        d.pin(id, true).unwrap();
        d.add("remove").unwrap();
        d.clear(false).unwrap();
        assert_eq!(d.list(&Search::new("", false, "all")).unwrap().len(), 1);
        d.clear(true).unwrap();
        assert!(d.list(&Search::new("", false, "all")).unwrap().is_empty());
    }
    #[test]
    fn migrate_v1_and_reject_future() {
        let path = std::env::temp_dir().join(format!("cv-migration-{}.db", std::process::id()));
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(include_str!("../migrations/001.sql"))
                .unwrap();
            c.execute("INSERT INTO clipboard_items(kind,text,hash,first_copied,last_copied) VALUES('text','old data','old',1,1)",[]).unwrap();
        }
        let d = Database::open(&path).unwrap();
        assert_eq!(d.list(&Search::new("old", false, "all")).unwrap().len(), 1);
        d.0.execute_batch("PRAGMA user_version=999").unwrap();
        drop(d);
        assert!(Database::open(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn images_retention_and_offset() {
        let mut d = db();
        d.save_settings(&crate::settings::Settings {
            max_items: 10,
            ..Default::default()
        })
        .unwrap();
        for i in 0..20 {
            d.add_image(&format!("{i:064x}.png"), &format!("{i:064x}"))
                .unwrap();
        }
        assert_eq!(d.list(&Search::new("", false, "image")).unwrap().len(), 10);
        assert!(d
            .list(&Search {
                offset: 200,
                ..Search::new("", false, "all")
            })
            .unwrap()
            .is_empty());
    }
    #[test]
    fn source_metadata_and_collection_management() {
        let mut d = db();
        d.add("source test").unwrap();
        d.record_source(
            blake3::hash(b"source test").to_hex().as_str(),
            Some("editor.exe"),
        )
        .unwrap();
        assert_eq!(
            d.list(&Search::new("", false, "all")).unwrap()[0]
                .source
                .as_deref(),
            Some("editor.exe")
        );
        d.collection("Old").unwrap();
        let id = d.collections().unwrap()[0].id;
        d.rename_collection(id, "New").unwrap();
        assert_eq!(d.collections().unwrap()[0].name, "New");
        d.delete_collection(id).unwrap();
        assert!(d.collections().unwrap().is_empty());
        assert_eq!(d.list(&Search::new("", false, "all")).unwrap().len(), 1);
    }
}
