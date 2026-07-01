use rusqlite::{Connection, OptionalExtension};

pub fn get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .optional()
}

pub fn set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO app_settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [key, value],
    )?;
    Ok(())
}

pub fn theme(conn: &Connection) -> rusqlite::Result<String> {
    Ok(get(conn, "theme")?.unwrap_or_else(|| "system".to_string()))
}

pub fn streak_rule(conn: &Connection) -> rusqlite::Result<String> {
    Ok(get(conn, "streak_rule")?.unwrap_or_else(|| "focused".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    fn conn() -> rusqlite::Connection { let c = db::open(":memory:").unwrap(); db::migrate(&c).unwrap(); c }
    #[test]
    fn defaults_then_override() {
        let c = conn();
        assert_eq!(theme(&c).unwrap(), "system");
        assert_eq!(streak_rule(&c).unwrap(), "focused");
        set(&c, "theme", "dark").unwrap();
        set(&c, "theme", "light").unwrap(); // upsert
        assert_eq!(theme(&c).unwrap(), "light");
        assert_eq!(get(&c, "missing").unwrap(), None);
    }
}
