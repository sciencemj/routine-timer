use rusqlite::Connection;
use chrono::{DateTime, Utc};

use crate::core::model::FocusSession;
use crate::core::timer::CompletedSession;

pub fn insert(conn: &Connection, s: &CompletedSession) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO focus_session (routine_id, started_at, ended_at, seconds, completed)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            s.routine_id,
            s.started_at.to_rfc3339(),
            s.ended_at.to_rfc3339(),
            s.seconds,
            s.completed as i64,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusSession> {
    let started_str: String = row.get(2)?;
    let ended_str: String = row.get(3)?;
    let completed_int: i64 = row.get(5)?;
    Ok(FocusSession {
        id: row.get(0)?,
        routine_id: row.get(1)?,
        started_at: DateTime::parse_from_rfc3339(&started_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?
            .with_timezone(&Utc),
        ended_at: DateTime::parse_from_rfc3339(&ended_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                3,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?
            .with_timezone(&Utc),
        seconds: row.get(4)?,
        completed: completed_int != 0,
    })
}

pub fn all(conn: &Connection) -> rusqlite::Result<Vec<FocusSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, routine_id, started_at, ended_at, seconds, completed FROM focus_session ORDER BY started_at"
    )?;
    let rows = stmt.query_map([], map_row)?;
    rows.collect()
}

pub fn since(conn: &Connection, from_rfc3339: &str) -> rusqlite::Result<Vec<FocusSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, routine_id, started_at, ended_at, seconds, completed FROM focus_session WHERE started_at >= ?1 ORDER BY started_at"
    )?;
    let rows = stmt.query_map(rusqlite::params![from_rfc3339], map_row)?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::core::timer::CompletedSession;
    use chrono::{Utc, TimeZone};
    fn conn() -> rusqlite::Connection { let c = db::open(":memory:").unwrap(); db::migrate(&c).unwrap();
        c.execute("INSERT INTO routine (name,icon,target_seconds,sort_order,created_at) VALUES ('r','x',3600,1,'t')", []).unwrap(); c }
    #[test]
    fn insert_and_read_back() {
        let c = conn();
        let t = Utc.timestamp_opt(1_700_000_000,0).unwrap();
        let s = CompletedSession { routine_id: 1, started_at: t, ended_at: t, seconds: 300, completed: true };
        let id = insert(&c, &s).unwrap();
        assert!(id > 0);
        let rows = all(&c).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].seconds, 300);
        assert!(rows[0].completed);
        assert_eq!(rows[0].started_at, t);
    }
}
