use rusqlite::{params, Connection, OptionalExtension, Result};
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

         CREATE TABLE IF NOT EXISTS chat_usage (
             id                INTEGER PRIMARY KEY AUTOINCREMENT,
             conversation_id   INTEGER REFERENCES conversations(id) ON DELETE SET NULL,
             prompt_tokens     INTEGER,
             completion_tokens INTEGER,
             tokens_estimated  INTEGER NOT NULL DEFAULT 0,
             latency_ms        INTEGER NOT NULL,
             created_at        INTEGER NOT NULL DEFAULT (unixepoch())
         );
         CREATE INDEX IF NOT EXISTS idx_chat_usage_created ON chat_usage(created_at);

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
         CREATE INDEX IF NOT EXISTS idx_suggestions_session ON copilot_suggestions(session_id);

         CREATE TABLE IF NOT EXISTS copilot_presets (
             id                 TEXT PRIMARY KEY,
             name               TEXT NOT NULL,
             system_prompt      TEXT NOT NULL,
             response_format    TEXT NOT NULL,
             default_context_s  INTEGER NOT NULL,
             created_at         INTEGER NOT NULL DEFAULT (unixepoch())
         );",
    )?;
    // chat_usage predates `tokens_estimated` in dev builds; add it to older tables.
    let has_estimated = conn
        .prepare("SELECT 1 FROM pragma_table_info('chat_usage') WHERE name = 'tokens_estimated'")?
        .exists([])?;
    if !has_estimated {
        conn.execute(
            "ALTER TABLE chat_usage ADD COLUMN tokens_estimated INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    // copilot_sessions predates `summary` in dev builds; add it to older tables.
    let has_summary = conn
        .prepare("SELECT 1 FROM pragma_table_info('copilot_sessions') WHERE name = 'summary'")?
        .exists([])?;
    if !has_summary {
        conn.execute(
            "ALTER TABLE copilot_sessions ADD COLUMN summary TEXT",
            [],
        )?;
    }
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

// ── Chat usage ───────────────────────────────────────────────────────────────

/// Record one completed chat stream. `estimated` marks token counts guessed
/// from text length because the server reported no `usage`; `latency_ms` is
/// time to the first token.
pub fn record_chat_usage(
    db: &Db,
    conv_id: Option<i64>,
    prompt_tokens: i64,
    completion_tokens: i64,
    estimated: bool,
    latency_ms: i64,
) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO chat_usage(conversation_id, prompt_tokens, completion_tokens, tokens_estimated, latency_ms)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![conv_id, prompt_tokens, completion_tokens, estimated, latency_ms],
    )?;
    Ok(())
}

#[derive(Serialize, Debug, PartialEq)]
pub struct UsageStats {
    pub messages_today:     i64,
    pub messages_yesterday: i64,
    pub tokens_today:       i64,
    /// True when any of today's token counts are estimates, not server-reported.
    pub tokens_estimated:   bool,
    /// Mean time-to-first-token over the last 7 days; `None` with no samples.
    pub avg_latency_ms:     Option<i64>,
}

/// `today_start` is local midnight as a unix timestamp, supplied by the caller
/// because SQLite only knows UTC. Only user messages count as "messages".
pub fn get_usage_stats(db: &Db, today_start: i64) -> Result<UsageStats> {
    let conn = db.0.lock().unwrap();
    let day = 86_400;
    let user_msgs = |from: i64, to: i64| -> Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM messages
             WHERE role = 'user' AND created_at >= ?1 AND created_at < ?2",
            params![from, to],
            |r| r.get(0),
        )
    };
    let messages_today = user_msgs(today_start, i64::MAX)?;
    let messages_yesterday = user_msgs(today_start - day, today_start)?;
    let tokens_today = conn.query_row(
        "SELECT COALESCE(SUM(COALESCE(prompt_tokens, 0) + COALESCE(completion_tokens, 0)), 0)
         FROM chat_usage WHERE created_at >= ?1",
        params![today_start],
        |r| r.get(0),
    )?;
    let tokens_estimated: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM chat_usage WHERE created_at >= ?1 AND tokens_estimated = 1)",
        params![today_start],
        |r| r.get(0),
    )?;
    let avg_latency_ms: Option<f64> = conn.query_row(
        "SELECT AVG(latency_ms) FROM chat_usage WHERE created_at >= ?1",
        params![today_start - 6 * day],
        |r| r.get(0),
    )?;
    Ok(UsageStats {
        messages_today,
        messages_yesterday,
        tokens_today,
        tokens_estimated,
        avg_latency_ms: avg_latency_ms.map(|v| v.round() as i64),
    })
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
    pub summary:          Option<String>,
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
                COUNT(g.id) AS sug_count, s.summary
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
            summary:          r.get(6)?,
        })
    })?;
    rows.collect()
}

/// Full transcript text for a session, chunks joined in chronological order.
pub fn get_copilot_transcript_text(db: &Db, session_id: i64) -> Result<String> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT text FROM copilot_transcripts WHERE session_id = ?1 ORDER BY start_ms ASC",
    )?;
    let rows = stmt.query_map(params![session_id], |r| r.get::<_, String>(0))?;
    let mut parts = Vec::new();
    for row in rows {
        parts.push(row?);
    }
    Ok(parts.join(" "))
}

pub fn save_copilot_summary(db: &Db, session_id: i64, summary: &str) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "UPDATE copilot_sessions SET summary = ?2 WHERE id = ?1",
        params![session_id, summary],
    )?;
    Ok(())
}

// ── Copilot presets (user-defined, in addition to Rust builtins) ──────────────

#[derive(Serialize)]
pub struct CustomPresetRow {
    pub id:                String,
    pub name:               String,
    pub system_prompt:      String,
    pub response_format:    String,
    pub default_context_s:  i64,
}

pub fn create_copilot_preset(
    db: &Db,
    id: &str,
    name: &str,
    system_prompt: &str,
    response_format: &str,
    default_context_s: i64,
) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute(
        "INSERT INTO copilot_presets(id, name, system_prompt, response_format, default_context_s)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![id, name, system_prompt, response_format, default_context_s],
    )?;
    Ok(())
}

pub fn delete_copilot_preset(db: &Db, id: &str) -> Result<()> {
    let conn = db.0.lock().unwrap();
    conn.execute("DELETE FROM copilot_presets WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn list_copilot_presets(db: &Db) -> Result<Vec<CustomPresetRow>> {
    let conn = db.0.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, system_prompt, response_format, default_context_s
         FROM copilot_presets ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(CustomPresetRow {
            id:                 r.get(0)?,
            name:               r.get(1)?,
            system_prompt:      r.get(2)?,
            response_format:    r.get(3)?,
            default_context_s:  r.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn find_copilot_preset(db: &Db, id: &str) -> Result<Option<CustomPresetRow>> {
    let conn = db.0.lock().unwrap();
    conn.query_row(
        "SELECT id, name, system_prompt, response_format, default_context_s
         FROM copilot_presets WHERE id = ?1",
        params![id],
        |r| Ok(CustomPresetRow {
            id:                 r.get(0)?,
            name:               r.get(1)?,
            system_prompt:      r.get(2)?,
            response_format:    r.get(3)?,
            default_context_s:  r.get(4)?,
        }),
    ).optional()
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
    fn transcript_text_joins_chunks_in_chronological_order() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let sid = create_copilot_session(&db, "generic", 90, true).unwrap();
        insert_copilot_transcript(&db, sid, 2000, 3000, "world").unwrap();
        insert_copilot_transcript(&db, sid, 0, 1000, "hello").unwrap();
        assert_eq!(get_copilot_transcript_text(&db, sid).unwrap(), "hello world");
    }

    #[test]
    fn transcript_text_empty_when_nothing_saved() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let sid = create_copilot_session(&db, "generic", 90, false).unwrap();
        assert_eq!(get_copilot_transcript_text(&db, sid).unwrap(), "");
    }

    #[test]
    fn summary_roundtrips_through_list_copilot_sessions() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        let sid = create_copilot_session(&db, "generic", 90, true).unwrap();
        save_copilot_summary(&db, sid, "Ringkasan sesi").unwrap();
        let rows = list_copilot_sessions(&db).unwrap();
        let row = rows.iter().find(|r| r.id == sid).unwrap();
        assert_eq!(row.summary.as_deref(), Some("Ringkasan sesi"));
    }

    #[test]
    fn custom_preset_crud_roundtrips() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        create_copilot_preset(&db, "custom-1", "My Preset", "Be helpful.", "Bullets", 90).unwrap();

        let found = find_copilot_preset(&db, "custom-1").unwrap().unwrap();
        assert_eq!(found.name, "My Preset");

        let all = list_copilot_presets(&db).unwrap();
        assert_eq!(all.len(), 1);

        delete_copilot_preset(&db, "custom-1").unwrap();
        assert!(find_copilot_preset(&db, "custom-1").unwrap().is_none());
    }

    #[test]
    fn find_copilot_preset_returns_none_for_unknown_id() {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        assert!(find_copilot_preset(&db, "nonexistent").unwrap().is_none());
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

    const DAY: i64 = 86_400;
    const TODAY: i64 = 1_800_000_000;

    fn test_db() -> (tempfile::TempDir, Db) {
        let dir = tempdir().unwrap();
        let db = open(dir.path()).unwrap();
        (dir, db)
    }

    fn add_msg(db: &Db, conv: i64, role: &str, at: i64) {
        db.0.lock().unwrap().execute(
            "INSERT INTO messages(conversation_id, role, body, created_at) VALUES(?1, ?2, 'x', ?3)",
            params![conv, role, at],
        ).unwrap();
    }

    fn add_usage(db: &Db, prompt: Option<i64>, completion: Option<i64>, latency: i64, at: i64) {
        db.0.lock().unwrap().execute(
            "INSERT INTO chat_usage(prompt_tokens, completion_tokens, latency_ms, created_at)
             VALUES(?1, ?2, ?3, ?4)",
            params![prompt, completion, latency, at],
        ).unwrap();
    }

    #[test]
    fn stats_are_empty_without_data() {
        let (_d, db) = test_db();
        assert_eq!(
            get_usage_stats(&db, TODAY).unwrap(),
            UsageStats {
                messages_today: 0, messages_yesterday: 0, tokens_today: 0,
                tokens_estimated: false, avg_latency_ms: None,
            },
        );
    }

    #[test]
    fn counts_only_user_messages_per_day() {
        let (_d, db) = test_db();
        let conv = create_conversation(&db, "t").unwrap();
        add_msg(&db, conv, "user", TODAY + 10);
        add_msg(&db, conv, "assistant", TODAY + 11);
        add_msg(&db, conv, "user", TODAY - 10);
        add_msg(&db, conv, "user", TODAY - DAY - 10); // two days ago
        let s = get_usage_stats(&db, TODAY).unwrap();
        assert_eq!((s.messages_today, s.messages_yesterday), (1, 1));
    }

    #[test]
    fn sums_todays_tokens_and_tolerates_missing_usage() {
        let (_d, db) = test_db();
        add_usage(&db, Some(10), Some(20), 100, TODAY + 1);
        add_usage(&db, None, None, 100, TODAY + 2);
        add_usage(&db, Some(500), Some(500), 100, TODAY - 1); // yesterday
        assert_eq!(get_usage_stats(&db, TODAY).unwrap().tokens_today, 30);
    }

    #[test]
    fn flags_estimated_tokens_only_for_today() {
        let (_d, db) = test_db();
        record_chat_usage(&db, None, 5, 5, false, 10).unwrap();
        assert!(!get_usage_stats(&db, 0).unwrap().tokens_estimated);
        record_chat_usage(&db, None, 5, 5, true, 10).unwrap();
        let s = get_usage_stats(&db, 0).unwrap();
        assert!(s.tokens_estimated);
        assert_eq!(s.tokens_today, 20);
        // The estimated row is not from today's window.
        assert!(!get_usage_stats(&db, i64::MAX / 2).unwrap().tokens_estimated);
    }

    #[test]
    fn open_adds_tokens_estimated_to_an_older_chat_usage_table() {
        let dir = tempdir().unwrap();
        {
            let conn = Connection::open(dir.path().join("profile.db")).unwrap();
            conn.execute_batch(
                "CREATE TABLE chat_usage (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     conversation_id INTEGER,
                     prompt_tokens INTEGER,
                     completion_tokens INTEGER,
                     latency_ms INTEGER NOT NULL,
                     created_at INTEGER NOT NULL DEFAULT (unixepoch())
                 );
                 INSERT INTO chat_usage(prompt_tokens, completion_tokens, latency_ms) VALUES(1, 2, 3);",
            ).unwrap();
        }
        let db = open(dir.path()).unwrap();
        let s = get_usage_stats(&db, 0).unwrap();
        assert_eq!((s.tokens_today, s.tokens_estimated), (3, false));
        open(dir.path()).unwrap(); // reopening is idempotent
    }

    #[test]
    fn averages_latency_over_seven_days() {
        let (_d, db) = test_db();
        add_usage(&db, None, None, 100, TODAY + 1);
        add_usage(&db, None, None, 301, TODAY - 6 * DAY);
        add_usage(&db, None, None, 9_999, TODAY - 7 * DAY); // outside window
        assert_eq!(get_usage_stats(&db, TODAY).unwrap().avg_latency_ms, Some(201));
    }

    #[test]
    fn record_chat_usage_persists_a_row() {
        let (_d, db) = test_db();
        record_chat_usage(&db, None, 1, 2, false, 42).unwrap();
        let n: i64 = db.0.lock().unwrap()
            .query_row("SELECT COUNT(*) FROM chat_usage WHERE latency_ms = 42", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
}
