pub mod routines;
pub mod sessions;
pub mod settings;

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

/// "DB 리셋" — wipes user DATA (routines + focus sessions + any suspended
/// pomodoro block) but KEEPS preferences (theme / streak_rule /
/// day_start_hour rows in app_settings survive).
pub fn reset(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM focus_session", [])?;
    conn.execute("DELETE FROM routine", [])?;
    conn.execute("DELETE FROM app_settings WHERE key = 'pomo_states'", [])?;
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

    #[test]
    fn reset_clears_data_but_keeps_preferences() {
        let conn = open(":memory:").unwrap();
        migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO routine (name,icon,target_seconds,sort_order,created_at) VALUES ('r','x',3600,1,'t')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO focus_session (routine_id, started_at, ended_at, seconds, completed) VALUES (1,'a','b',300,1)",
            [],
        ).unwrap();
        conn.execute("INSERT INTO app_settings(key,value) VALUES('theme','dark')", []).unwrap();
        conn.execute("INSERT INTO app_settings(key,value) VALUES('streak_rule','any_completed')", []).unwrap();
        conn.execute("INSERT INTO app_settings(key,value) VALUES('day_start_hour','9')", []).unwrap();
        conn.execute("INSERT INTO app_settings(key,value) VALUES('pomo_states','{}')", []).unwrap();

        reset(&conn).unwrap();

        let routines: i64 = conn.query_row("SELECT count(*) FROM routine", [], |r| r.get(0)).unwrap();
        let sessions: i64 = conn.query_row("SELECT count(*) FROM focus_session", [], |r| r.get(0)).unwrap();
        assert_eq!(routines, 0);
        assert_eq!(sessions, 0);

        let theme: String = conn.query_row("SELECT value FROM app_settings WHERE key='theme'", [], |r| r.get(0)).unwrap();
        assert_eq!(theme, "dark");
        let rule: String = conn.query_row("SELECT value FROM app_settings WHERE key='streak_rule'", [], |r| r.get(0)).unwrap();
        assert_eq!(rule, "any_completed");
        let hour: String = conn.query_row("SELECT value FROM app_settings WHERE key='day_start_hour'", [], |r| r.get(0)).unwrap();
        assert_eq!(hour, "9");

        let pomo: i64 = conn.query_row("SELECT count(*) FROM app_settings WHERE key='pomo_states'", [], |r| r.get(0)).unwrap();
        assert_eq!(pomo, 0);
    }
}
