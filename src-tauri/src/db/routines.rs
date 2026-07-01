use rusqlite::{Connection, Result};
use crate::core::model::{NewRoutine, Routine};

fn map_row(row: &rusqlite::Row) -> Result<Routine> {
    Ok(Routine {
        id:               row.get(0)?,
        name:             row.get(1)?,
        icon:             row.get(2)?,
        color:            row.get(3)?,
        target_seconds:   row.get(4)?,
        pomodoro_enabled: row.get::<_, i64>(5)? != 0,
        focus_minutes:    row.get(6)?,
        break_minutes:    row.get(7)?,
        sort_order:       row.get(8)?,
        archived:         row.get::<_, i64>(9)? != 0,
        created_at:       row.get(10)?,
    })
}

pub fn create(conn: &Connection, r: &NewRoutine, created_at: &str) -> Result<Routine> {
    conn.execute(
        "INSERT INTO routine (name, icon, color, target_seconds, pomodoro_enabled,
                              focus_minutes, break_minutes, sort_order, archived, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                 COALESCE((SELECT MAX(sort_order) FROM routine), 0) + 1,
                 0, ?8)",
        rusqlite::params![
            r.name, r.icon, r.color, r.target_seconds,
            r.pomodoro_enabled as i64, r.focus_minutes, r.break_minutes,
            created_at,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get(conn, id).map(|opt| opt.expect("just inserted row must exist"))
}

pub fn list(conn: &Connection) -> Result<Vec<Routine>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, icon, color, target_seconds, pomodoro_enabled,
                focus_minutes, break_minutes, sort_order, archived, created_at
         FROM routine WHERE archived = 0 ORDER BY sort_order",
    )?;
    let rows = stmt.query_map([], map_row)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Routine>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, icon, color, target_seconds, pomodoro_enabled,
                focus_minutes, break_minutes, sort_order, archived, created_at
         FROM routine WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map([id], map_row)?;
    match rows.next() {
        Some(r) => r.map(Some),
        None    => Ok(None),
    }
}

pub fn update(conn: &Connection, r: &Routine) -> Result<()> {
    conn.execute(
        "UPDATE routine SET name=?1, icon=?2, color=?3, target_seconds=?4,
                            pomodoro_enabled=?5, focus_minutes=?6, break_minutes=?7,
                            sort_order=?8, archived=?9
         WHERE id=?10",
        rusqlite::params![
            r.name, r.icon, r.color, r.target_seconds,
            r.pomodoro_enabled as i64, r.focus_minutes, r.break_minutes,
            r.sort_order, r.archived as i64, r.id,
        ],
    )?;
    Ok(())
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool) -> Result<()> {
    conn.execute(
        "UPDATE routine SET archived=?1 WHERE id=?2",
        rusqlite::params![archived as i64, id],
    )?;
    Ok(())
}

pub fn reorder(conn: &Connection, ordered_ids: &[i64]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for (i, &id) in ordered_ids.iter().enumerate() {
        tx.execute(
            "UPDATE routine SET sort_order=?1 WHERE id=?2",
            rusqlite::params![i as i64 + 1, id],
        )?;
    }
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::core::model::NewRoutine;

    fn conn() -> rusqlite::Connection {
        let c = db::open(":memory:").unwrap();
        db::migrate(&c).unwrap();
        c
    }

    fn nr(name: &str) -> NewRoutine {
        NewRoutine {
            name: name.into(),
            icon: "🎯".into(),
            color: None,
            target_seconds: 3600,
            pomodoro_enabled: true,
            focus_minutes: 25,
            break_minutes: 5,
        }
    }

    #[test]
    fn create_list_update_archive() {
        let c = conn();
        let a = create(&c, &nr("딥워크"), "2026-07-01T00:00:00Z").unwrap();
        let b = create(&c, &nr("외국어"), "2026-07-01T00:00:00Z").unwrap();
        assert_eq!(list(&c).unwrap().len(), 2);
        assert!(b.sort_order > a.sort_order);
        let mut a2 = a.clone();
        a2.name = "딥워크2".into();
        a2.target_seconds = 7200;
        update(&c, &a2).unwrap();
        assert_eq!(get(&c, a.id).unwrap().unwrap().name, "딥워크2");
        set_archived(&c, a.id, true).unwrap();
        assert_eq!(list(&c).unwrap().len(), 1);
    }

    #[test]
    fn reorder_assigns_sort_order() {
        let c = conn();
        let a = create(&c, &nr("a"), "t").unwrap();
        let b = create(&c, &nr("b"), "t").unwrap();
        reorder(&c, &[b.id, a.id]).unwrap();
        let ids: Vec<i64> = list(&c).unwrap().iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![b.id, a.id]);
    }
}
