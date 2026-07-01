use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use crate::core::model::{Mode, NewRoutine, Routine};
use crate::core::timer::{TimerConfig, TimerSnapshot};
use crate::state::AppState;

// ── Routine commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn routines_list(state: State<'_, Mutex<AppState>>) -> Result<Vec<Routine>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    crate::db::routines::list(&s.db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn routine_create(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    new: NewRoutine,
) -> Result<Routine, String> {
    let routine = {
        let s = state.lock().map_err(|e| e.to_string())?;
        let created_at = chrono::Utc::now().to_rfc3339();
        crate::db::routines::create(&s.db, &new, &created_at)
            .map_err(|e| e.to_string())?
    }; // guard dropped
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    Ok(routine)
}

#[tauri::command]
pub fn routine_update(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    routine: Routine,
) -> Result<(), String> {
    {
        let s = state.lock().map_err(|e| e.to_string())?;
        crate::db::routines::update(&s.db, &routine).map_err(|e| e.to_string())?;
    } // guard dropped
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn routine_delete(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    id: i64,
) -> Result<(), String> {
    {
        let s = state.lock().map_err(|e| e.to_string())?;
        crate::db::routines::set_archived(&s.db, id, true).map_err(|e| e.to_string())?;
    } // guard dropped
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn routine_reorder(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    ordered_ids: Vec<i64>,
) -> Result<(), String> {
    {
        let s = state.lock().map_err(|e| e.to_string())?;
        crate::db::routines::reorder(&s.db, &ordered_ids).map_err(|e| e.to_string())?;
    } // guard dropped
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Timer config helper ────────────────────────────────────────────────────

pub fn build_config(routine: &Routine, already_done: i64) -> TimerConfig {
    TimerConfig {
        routine_id: routine.id,
        mode: if routine.pomodoro_enabled { Mode::Pomodoro } else { Mode::Continuous },
        focus_secs: routine.focus_minutes * 60,
        break_secs: routine.break_minutes * 60,
        target_secs: routine.target_seconds,
        already_done_secs: already_done.min(routine.target_seconds),
    }
}

// ── Timer commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn timer_start(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    routine_id: i64,
) -> Result<TimerSnapshot, String> {
    let snap = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let routine = crate::db::routines::get(&s.db, routine_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("routine {} not found", routine_id))?;
        let tz = *chrono::Local::now().offset();
        let day = crate::core::stats::day_of(chrono::Utc::now(), tz);
        let sessions = crate::db::sessions::all(&s.db).map_err(|e| e.to_string())?;
        let already_done = crate::core::stats::seconds_per_routine(&sessions, day, tz)
            .get(&routine_id)
            .copied()
            .unwrap_or(0);
        let cfg = build_config(&routine, already_done);
        s.engine.start(cfg);
        s.current_routine_name = Some(routine.name.clone());
        s.engine.snapshot()
    }; // guard dropped
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(snap)
}

#[tauri::command]
pub fn timer_pause(state: State<'_, Mutex<AppState>>) -> Result<TimerSnapshot, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.engine.pause();
    Ok(s.engine.snapshot())
}

#[tauri::command]
pub fn timer_resume(state: State<'_, Mutex<AppState>>) -> Result<TimerSnapshot, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.engine.resume();
    Ok(s.engine.snapshot())
}

#[tauri::command]
pub fn timer_skip_break(state: State<'_, Mutex<AppState>>) -> Result<TimerSnapshot, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.engine.skip_break();
    Ok(s.engine.snapshot())
}

#[tauri::command]
pub fn timer_stop(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let snap = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if let Some(done) = s.engine.stop() {
            crate::db::sessions::insert(&s.db, &done).map_err(|e| e.to_string())?;
        }
        s.current_routine_name = None;
        s.engine.snapshot()
    }; // guard dropped
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn timer_switch(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    routine_id: i64,
) -> Result<TimerSnapshot, String> {
    let snap = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        // Stop and persist current session if active
        if let Some(done) = s.engine.stop() {
            crate::db::sessions::insert(&s.db, &done).map_err(|e| e.to_string())?;
        }
        // Start the new routine
        let routine = crate::db::routines::get(&s.db, routine_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("routine {} not found", routine_id))?;
        let tz = *chrono::Local::now().offset();
        let day = crate::core::stats::day_of(chrono::Utc::now(), tz);
        let sessions = crate::db::sessions::all(&s.db).map_err(|e| e.to_string())?;
        let already_done = crate::core::stats::seconds_per_routine(&sessions, day, tz)
            .get(&routine_id)
            .copied()
            .unwrap_or(0);
        let cfg = build_config(&routine, already_done);
        s.engine.start(cfg);
        s.current_routine_name = Some(routine.name.clone());
        s.engine.snapshot()
    }; // guard dropped
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(snap)
}

#[tauri::command]
pub fn timer_get_state(state: State<'_, Mutex<AppState>>) -> Result<TimerSnapshot, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.engine.snapshot())
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_config_maps_mode_and_seconds() {
        let r = Routine { id: 3, name: "x".into(), icon: "x".into(), color: None, target_seconds: 3600,
            pomodoro_enabled: false, focus_minutes: 25, break_minutes: 5, sort_order: 1, archived: false, created_at: "".into() };
        let c = build_config(&r, 600);
        assert_eq!(c.mode, Mode::Continuous);
        assert_eq!(c.focus_secs, 1500);
        assert_eq!(c.already_done_secs, 600);
    }
}
