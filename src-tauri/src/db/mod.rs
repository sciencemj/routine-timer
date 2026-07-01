pub mod routines;
pub mod sessions;

use rusqlite::Connection;

pub fn open(path: &str) -> rusqlite::Result<Connection> {
    Connection::open(path)
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS routine (
           id INTEGER PRIMARY KEY, name TEXT NOT NULL, icon TEXT NOT NULL,
           color TEXT, target_seconds INTEGER NOT NULL,
           pomodoro_enabled INTEGER NOT NULL DEFAULT 1,
           focus_minutes INTEGER NOT NULL DEFAULT 25,
           break_minutes INTEGER NOT NULL DEFAULT 5,
           sort_order INTEGER NOT NULL, archived INTEGER NOT NULL DEFAULT 0,
           created_at TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS focus_session (
           id INTEGER PRIMARY KEY, routine_id INTEGER NOT NULL REFERENCES routine(id),
           started_at TEXT NOT NULL, ended_at TEXT NOT NULL,
           seconds INTEGER NOT NULL, completed INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS idx_session_started ON focus_session(started_at);
         CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
    let v: i64 = conn.query_row("SELECT COALESCE(MAX(version),0) FROM schema_version", [], |r| r.get(0)).unwrap_or(0);
    if v < 1 { conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrate_creates_tables_idempotently() {
        let conn = open(":memory:").unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap(); // idempotent
        let n: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('routine','focus_session','app_settings')",
            [], |r| r.get(0)).unwrap();
        assert_eq!(n, 3);
    }
}
