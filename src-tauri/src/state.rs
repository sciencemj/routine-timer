use crate::core::timer::TimerEngine;

pub struct AppState {
    pub engine: TimerEngine,
    pub db: rusqlite::Connection,
    pub current_routine_name: Option<String>, // for the tray title "딥워크 24:13"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::clock::SystemClock;
    use crate::core::timer::TimerEngine;
    use crate::core::model::NewRoutine;

    #[test]
    fn appstate_can_create_and_list_routines() {
        let db = crate::db::open(":memory:").unwrap();
        crate::db::migrate(&db).unwrap();
        let mut st = AppState {
            engine: TimerEngine::new(Box::new(SystemClock)),
            db,
            current_routine_name: None,
        };
        crate::db::routines::create(
            &st.db,
            &NewRoutine {
                name: "딥워크".into(),
                icon: "🎯".into(),
                color: None,
                target_seconds: 3600,
                pomodoro_enabled: true,
                focus_minutes: 25,
                break_minutes: 5,
            },
            "2026-07-01T00:00:00Z",
        )
        .unwrap();
        assert_eq!(crate::db::routines::list(&st.db).unwrap().len(), 1);
        let _ = &mut st.engine; // engine present
    }
}
