//! Import the previous app's owned data before opening the new database.
//! Webview cookies/caches are deliberately left to the platform webview.

use rusqlite::{Connection, DatabaseName, OpenFlags};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

const LEGACY_ID: &str = "com.noscreen.app";
const CURRENT_ID: &str = "com.veilo.app";
const MARKER: &str = ".noscreen-migration-v1";
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn migrate(destination: &Path) -> Result<()> {
    // Do not import from an unexpected location if the app ID changes again.
    if destination.file_name().and_then(|name| name.to_str()) != Some(CURRENT_ID) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unexpected Veilo data directory",
        )
        .into());
    }
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing app data parent directory",
        )
    })?;
    import_data(&parent.join(LEGACY_ID), destination)
}

fn import_data(source: &Path, destination: &Path) -> Result<()> {
    if present(&destination.join(MARKER))? {
        return Ok(());
    }
    fs::create_dir_all(destination)?;

    let config = source.join("config.json");
    let target_config = destination.join("config.json");
    if !present(&target_config)? && present(&config)? {
        let bytes = fs::read(&config)?;
        // An invalid legacy config must not silently become a fresh profile.
        let _: crate::config::Config = serde_json::from_slice(&bytes)?;
        let mut staged = NamedTempFile::new_in(destination)?;
        staged.write_all(&bytes)?;
        publish(staged, &target_config)?;
    }

    let database = source.join("profile.db");
    let target_database = destination.join("profile.db");
    if !present(&target_database)? && present(&database)? {
        let staged = NamedTempFile::new_in(destination)?;
        // SQLite's backup API includes committed WAL pages. A filesystem copy
        // of profile.db alone can silently lose recent chats/settings.
        let old = Connection::open_with_flags(&database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        old.backup(DatabaseName::Main, staged.path(), None)?;
        publish(staged, &target_database)?;
    }

    // Also mark fresh installs: old data appearing later must not be imported.
    // If a previous attempt failed, retry only the files still missing.
    let mut marker = NamedTempFile::new_in(destination)?;
    marker.write_all(b"Veilo data migration completed\n")?;
    publish(marker, &destination.join(MARKER))
}

fn present(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn publish(staged: NamedTempFile, target: &Path) -> Result<()> {
    staged.as_file().sync_all()?;
    // Atomic publication on the same filesystem; never replace an existing
    // file, including one created by another instance during this migration.
    match staged.persist_noclobber(target) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn legacy_profile(root: &Path) -> std::path::PathBuf {
        let source = root.join(LEGACY_ID);
        fs::create_dir_all(&source).unwrap();
        let config = crate::config::Config {
            opacity: 0.65,
            ..Default::default()
        };
        crate::config::write_config(&source, &config).unwrap();
        let db = Connection::open(source.join("profile.db")).unwrap();
        db.execute_batch(
            "CREATE TABLE profile (key TEXT PRIMARY KEY, value TEXT);
            INSERT INTO profile VALUES ('api_key', 'test-only-key');
            CREATE TABLE messages (body TEXT);
            INSERT INTO messages VALUES ('previous conversation');",
        )
        .unwrap();
        source
    }

    #[test]
    fn imports_config_and_database_without_removing_originals() {
        let root = TempDir::new().unwrap();
        let source = legacy_profile(root.path());
        let destination = root.path().join(CURRENT_ID);
        migrate(&destination).unwrap();
        assert_eq!(
            fs::read(source.join("config.json")).unwrap(),
            fs::read(destination.join("config.json")).unwrap()
        );
        let db = Connection::open(destination.join("profile.db")).unwrap();
        let key: String = db
            .query_row(
                "SELECT value FROM profile WHERE key = 'api_key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(key, "test-only-key");
        let body: String = db
            .query_row("SELECT body FROM messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(body, "previous conversation");
        assert!(source.join("profile.db").is_file());
        assert!(destination.join(MARKER).is_file());
    }

    #[test]
    fn includes_uncheckpointed_wal_data() {
        let root = TempDir::new().unwrap();
        let source = legacy_profile(root.path());
        let db = Connection::open(source.join("profile.db")).unwrap();
        db.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
            INSERT INTO messages VALUES ('latest WAL message');",
        )
        .unwrap();
        assert!(source.join("profile.db-wal").exists());
        let destination = root.path().join(CURRENT_ID);
        migrate(&destination).unwrap();
        let imported = Connection::open(destination.join("profile.db")).unwrap();
        let count: i64 = imported
            .query_row("SELECT count(*) FROM messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn preserves_existing_veilo_data() {
        let root = TempDir::new().unwrap();
        legacy_profile(root.path());
        let destination = root.path().join(CURRENT_ID);
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("config.json"), b"existing config").unwrap();
        fs::write(destination.join("profile.db"), b"existing database").unwrap();
        migrate(&destination).unwrap();
        assert_eq!(
            fs::read(destination.join("config.json")).unwrap(),
            b"existing config"
        );
        assert_eq!(
            fs::read(destination.join("profile.db")).unwrap(),
            b"existing database"
        );
    }

    #[test]
    fn corrupt_database_can_be_retried_without_replacing_imported_config() {
        let root = TempDir::new().unwrap();
        let source = legacy_profile(root.path());
        fs::write(source.join("profile.db"), b"not sqlite").unwrap();
        let destination = root.path().join(CURRENT_ID);
        assert!(migrate(&destination).is_err());
        assert!(!destination.join("profile.db").exists());
        assert!(!destination.join(MARKER).exists());
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 1);
        fs::write(destination.join("config.json"), b"already imported").unwrap();
        fs::remove_file(source.join("profile.db")).unwrap();
        legacy_profile(root.path());
        migrate(&destination).unwrap();
        assert_eq!(
            fs::read(destination.join("config.json")).unwrap(),
            b"already imported"
        );
        assert!(destination.join("profile.db").exists());
    }

    #[test]
    fn invalid_config_aborts_before_opening_a_new_database() {
        let root = TempDir::new().unwrap();
        let source = legacy_profile(root.path());
        fs::write(source.join("config.json"), b"invalid json").unwrap();
        let destination = root.path().join(CURRENT_ID);
        assert!(migrate(&destination).is_err());
        assert_eq!(fs::read_dir(destination).unwrap().count(), 0);
    }

    #[test]
    fn completion_marker_prevents_importing_again_after_reset() {
        let root = TempDir::new().unwrap();
        legacy_profile(root.path());
        let destination = root.path().join(CURRENT_ID);
        migrate(&destination).unwrap();
        fs::remove_file(destination.join("profile.db")).unwrap();
        migrate(&destination).unwrap();
        assert!(!destination.join("profile.db").exists());
    }

    #[test]
    fn fresh_install_does_not_import_legacy_data_that_appears_later() {
        let root = TempDir::new().unwrap();
        let destination = root.path().join(CURRENT_ID);
        migrate(&destination).unwrap();
        legacy_profile(root.path());
        migrate(&destination).unwrap();
        assert!(!destination.join("profile.db").exists());
    }
}
