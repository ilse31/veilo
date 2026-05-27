use rusqlite::{params, Connection, Result};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

pub fn open(app_data_dir: &Path) -> Result<Db> {
    std::fs::create_dir_all(app_data_dir).ok();
    let conn = Connection::open(app_data_dir.join("profile.db"))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS profile (
             key   TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );

         CREATE TABLE IF NOT EXISTS conversations (
             id         INTEGER PRIMARY KEY AUTOINCREMENT,
             title      TEXT    NOT NULL DEFAULT 'Obrolan baru',
             created_at INTEGER NOT NULL DEFAULT (unixepoch())
         );

         CREATE TABLE IF NOT EXISTS messages (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
             role            TEXT    NOT NULL,
             body            TEXT    NOT NULL,
             created_at      INTEGER NOT NULL DEFAULT (unixepoch())
         );",
    )?;
    Ok(Db(Mutex::new(conn)))
}

// ── Profile key-value ────────────────────────────────────────────────────────

pub fn get_value(db: &Db, key: &str) -> Result<Option<String>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare("SELECT value FROM profile WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    Ok(rows.next()?.map(|r| r.get(0).unwrap()))
}

pub fn set_value(db: &Db, key: &str, value: &str) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO profile(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// ── Conversations ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ConvRow {
    pub id:        i64,
    pub title:     String,
    pub msg_count: i64,
    pub created_at: i64,
}

pub fn list_conversations(db: &Db) -> Result<Vec<ConvRow>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT c.id, c.title, COUNT(m.id) AS msg_count, c.created_at
         FROM conversations c
         LEFT JOIN messages m ON m.conversation_id = c.id
         GROUP BY c.id
         ORDER BY c.id DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ConvRow {
            id:         r.get(0)?,
            title:      r.get(1)?,
            msg_count:  r.get(2)?,
            created_at: r.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn create_conversation(db: &Db, title: &str) -> Result<i64> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO conversations(title) VALUES(?1)",
        params![title],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_conversation_title(db: &Db, conv_id: i64, title: &str) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "UPDATE conversations SET title = ?1 WHERE id = ?2",
        params![title, conv_id],
    )?;
    Ok(())
}

pub fn delete_conversation(db: &Db, conv_id: i64) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute("DELETE FROM conversations WHERE id = ?1", params![conv_id])?;
    Ok(())
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct MsgRow {
    pub id:   i64,
    pub role: String,
    pub body: String,
}

pub fn get_messages(db: &Db, conv_id: i64) -> Result<Vec<MsgRow>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, role, body FROM messages
         WHERE conversation_id = ?1
         ORDER BY id ASC",
    )?;
    let rows = stmt.query_map(params![conv_id], |r| {
        Ok(MsgRow { id: r.get(0)?, role: r.get(1)?, body: r.get(2)? })
    })?;
    rows.collect()
}

pub fn append_message(db: &Db, conv_id: i64, role: &str, body: &str) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO messages(conversation_id, role, body) VALUES(?1, ?2, ?3)",
        params![conv_id, role, body],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn set_and_get_value() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        assert_eq!(get_value(&db, "name").unwrap(), None);
        set_value(&db, "name", "Adi").unwrap();
        assert_eq!(get_value(&db, "name").unwrap(), Some("Adi".into()));
    }

    #[test]
    fn upsert_overwrites() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        set_value(&db, "name", "Adi").unwrap();
        set_value(&db, "name", "Budi").unwrap();
        assert_eq!(get_value(&db, "name").unwrap(), Some("Budi".into()));
    }
}
