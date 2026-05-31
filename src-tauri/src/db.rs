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
         );

         CREATE TABLE IF NOT EXISTS copilot_sessions (
             id               INTEGER PRIMARY KEY AUTOINCREMENT,
             preset_id        TEXT    NOT NULL,
             started_at       INTEGER NOT NULL DEFAULT (unixepoch()),
             ended_at         INTEGER,
             save_transcript  INTEGER NOT NULL DEFAULT 1,
             context_window_s INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS copilot_transcripts (
             id          INTEGER PRIMARY KEY AUTOINCREMENT,
             session_id  INTEGER NOT NULL REFERENCES copilot_sessions(id) ON DELETE CASCADE,
             start_ms    INTEGER NOT NULL,
             end_ms      INTEGER NOT NULL,
             text        TEXT    NOT NULL,
             created_at  INTEGER NOT NULL DEFAULT (unixepoch())
         );
         CREATE INDEX IF NOT EXISTS idx_transcripts_session ON copilot_transcripts(session_id);

         CREATE TABLE IF NOT EXISTS copilot_suggestions (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             session_id      INTEGER NOT NULL REFERENCES copilot_sessions(id) ON DELETE CASCADE,
             prompt_context  TEXT    NOT NULL,
             response        TEXT    NOT NULL,
             response_format TEXT    NOT NULL,
             created_at      INTEGER NOT NULL DEFAULT (unixepoch())
         );
         CREATE INDEX IF NOT EXISTS idx_suggestions_session ON copilot_suggestions(session_id);",
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

// ── Copilot ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CopilotSessionRow {
    pub id:               i64,
    pub preset_id:        String,
    pub started_at:       i64,
    pub ended_at:         Option<i64>,
    pub context_window_s: i64,
    pub suggestion_count: i64,
}

pub fn create_copilot_session(
    db: &Db,
    preset_id: &str,
    context_window_s: i64,
    save_transcript: bool,
) -> Result<i64> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO copilot_sessions(preset_id, context_window_s, save_transcript)
         VALUES(?1, ?2, ?3)",
        params![preset_id, context_window_s, save_transcript as i64],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn end_copilot_session(db: &Db, session_id: i64) -> Result<()> {
    let conn = db.0.lock().unwrap();
    let changed = conn.execute(
        "UPDATE copilot_sessions SET ended_at = unixepoch() WHERE id = ?1",
        params![session_id],
    )?;
    if changed == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

pub fn purge_copilot_session_data(db: &Db, session_id: i64) -> Result<()> {
    let conn = db.0.lock().unwrap();
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM copilot_transcripts WHERE session_id = ?1", params![session_id])?;
    tx.execute("DELETE FROM copilot_suggestions WHERE session_id = ?1", params![session_id])?;
    tx.commit()?;
    Ok(())
}

pub fn insert_copilot_transcript(
    db: &Db,
    session_id: i64,
    start_ms: u64,
    end_ms: u64,
    text: &str,
) -> Result<()> {
    let start_i = i64::try_from(start_ms)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let end_i = i64::try_from(end_ms)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO copilot_transcripts(session_id, start_ms, end_ms, text)
         VALUES(?1, ?2, ?3, ?4)",
        params![session_id, start_i, end_i, text],
    )?;
    Ok(())
}

pub fn insert_copilot_suggestion(
    db: &Db,
    session_id: i64,
    prompt_context: &str,
    response: &str,
    response_format: &str,
) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO copilot_suggestions(session_id, prompt_context, response, response_format)
         VALUES(?1, ?2, ?3, ?4)",
        params![session_id, prompt_context, response, response_format],
    )?;
    Ok(())
}

pub fn list_copilot_sessions(db: &Db) -> Result<Vec<CopilotSessionRow>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT s.id, s.preset_id, s.started_at, s.ended_at, s.context_window_s,
                COUNT(g.id) AS sug_count
         FROM copilot_sessions s
         LEFT JOIN copilot_suggestions g ON g.session_id = s.id
         GROUP BY s.id
         ORDER BY s.id DESC
         -- NOTE: capped at 50; v2 should add pagination.
         LIMIT 50",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(CopilotSessionRow {
            id:               r.get(0)?,
            preset_id:        r.get(1)?,
            started_at:       r.get(2)?,
            ended_at:         r.get(3)?,
            context_window_s: r.get(4)?,
            suggestion_count: r.get(5)?,
        })
    })?;
    rows.collect()
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

    #[test]
    fn copilot_tables_exist() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let conn = db.0.lock().unwrap();
        for name in ["copilot_sessions", "copilot_transcripts", "copilot_suggestions"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [name],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "{name} table should be created by open()");
        }
    }

    #[test]
    fn copilot_cascade_delete_works() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let conn = db.0.lock().unwrap();
        conn.execute(
            "INSERT INTO copilot_sessions(preset_id, context_window_s) VALUES('generic', 90)",
            [],
        )
        .unwrap();
        let sid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO copilot_transcripts(session_id, start_ms, end_ms, text)
             VALUES(?1, 0, 1000, 'hello')",
            [sid],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO copilot_suggestions(session_id, prompt_context, response, response_format)
             VALUES(?1, 'ctx', 'resp', 'Bullets')",
            [sid],
        )
        .unwrap();

        conn.execute("DELETE FROM copilot_sessions WHERE id = ?1", [sid]).unwrap();

        let t_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM copilot_transcripts WHERE session_id = ?1", [sid], |r| r.get(0))
            .unwrap();
        let s_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM copilot_suggestions WHERE session_id = ?1", [sid], |r| r.get(0))
            .unwrap();
        assert_eq!(t_count, 0, "transcripts should cascade-delete");
        assert_eq!(s_count, 0, "suggestions should cascade-delete");
    }

    #[test]
    fn create_and_end_copilot_session() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let sid = create_copilot_session(&db, "generic", 90, true).unwrap();
        assert!(sid > 0);
        end_copilot_session(&db, sid).unwrap();
        let rows = list_copilot_sessions(&db).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, sid);
        assert_eq!(rows[0].preset_id, "generic");
        assert!(rows[0].ended_at.is_some());
    }

    #[test]
    fn purge_removes_transcripts_and_suggestions() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let sid = create_copilot_session(&db, "generic", 90, false).unwrap();
        insert_copilot_transcript(&db, sid, 0, 1000, "hi").unwrap();
        insert_copilot_suggestion(&db, sid, "ctx", "resp", "Bullets").unwrap();
        purge_copilot_session_data(&db, sid).unwrap();
        let conn = db.0.lock().unwrap();
        let t: i64 = conn.query_row("SELECT COUNT(*) FROM copilot_transcripts WHERE session_id = ?1", [sid], |r| r.get(0)).unwrap();
        let s: i64 = conn.query_row("SELECT COUNT(*) FROM copilot_suggestions WHERE session_id = ?1", [sid], |r| r.get(0)).unwrap();
        assert_eq!(t, 0);
        assert_eq!(s, 0);
    }

    #[test]
    fn insert_transcript_rejects_overflow() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let sid = create_copilot_session(&db, "generic", 90, true).unwrap();
        let res = insert_copilot_transcript(&db, sid, u64::MAX, u64::MAX, "x");
        assert!(res.is_err(), "u64::MAX should fail conversion to i64");
    }

    #[test]
    fn end_session_errors_on_missing_id() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let res = end_copilot_session(&db, 99999);
        assert!(res.is_err(), "ending a nonexistent session should error");
    }
}
