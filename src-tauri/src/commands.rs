use std::collections::HashMap;
use std::sync::Mutex;
use chrono::NaiveDate;
use chrono::FixedOffset;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use crate::core::model::{FocusSession, Mode, NewRoutine, Routine, StreakRule};
use crate::core::timer::{ResumeState, TimerConfig, TimerSnapshot, TimerState};
use crate::core::stats;
use crate::state::AppState;

// ── Stats types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TodayStats {
    pub total_secs: i64,
    pub completed: usize,
    pub routine_count: usize,
    pub remaining_secs: i64,
    pub streak: u32,
    pub best_streak: u32,
    pub per_routine: HashMap<i64, i64>,
}

pub fn today_stats(
    routines: &[Routine],
    sessions: &[FocusSession],
    rule: StreakRule,
    today: NaiveDate,
    tz: FixedOffset,
) -> TodayStats {
    TodayStats {
        total_secs: stats::today_total(sessions, today, tz),
        completed: stats::completed_count(routines, sessions, today, tz),
        routine_count: routines.iter().filter(|r| !r.archived).count(),
        remaining_secs: stats::remaining_total(routines, sessions, today, tz),
        streak: stats::streak(routines, sessions, rule, today, tz),
        best_streak: stats::max_streak(routines, sessions, rule, today, tz),
        per_routine: stats::seconds_per_routine(sessions, today, tz),
    }
}

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

// ── Suspended-pomodoro persistence ─────────────────────────────────────────
//
// A single app_settings row ("pomo_states") holds a JSON map of
// routine_id (string) -> ResumeState, so an interrupted pomodoro block can
// RESUME (remaining time, phase, session index) instead of restarting at 25:00.

const POMO_STATES_KEY: &str = "pomo_states";

/// Load the suspended-pomodoro map. Returns empty on missing row or parse error.
/// JSON keys are strings; they are converted back to i64 routine ids.
fn load_pomo_states(db: &rusqlite::Connection) -> HashMap<i64, ResumeState> {
    let raw = match crate::db::settings::get(db, POMO_STATES_KEY) {
        Ok(Some(s)) => s,
        _ => return HashMap::new(),
    };
    let parsed: HashMap<String, ResumeState> = match serde_json::from_str(&raw) {
        Ok(m) => m,
        Err(_) => return HashMap::new(),
    };
    parsed
        .into_iter()
        .filter_map(|(k, v)| k.parse::<i64>().ok().map(|id| (id, v)))
        .collect()
}

/// Persist the suspended-pomodoro map with string routine-id keys.
fn save_pomo_states(db: &rusqlite::Connection, map: &HashMap<i64, ResumeState>) -> Result<(), String> {
    let stringed: HashMap<String, &ResumeState> =
        map.iter().map(|(k, v)| (k.to_string(), v)).collect();
    let json = serde_json::to_string(&stringed).map_err(|e| e.to_string())?;
    crate::db::settings::set(db, POMO_STATES_KEY, &json).map_err(|e| e.to_string())
}

/// Save the current pomodoro block so it can resume later. No-op unless the
/// snapshot is a non-Idle Pomodoro session.
fn suspend_pomo(db: &rusqlite::Connection, routine_id: i64, snap: &TimerSnapshot) -> Result<(), String> {
    if snap.mode == Mode::Pomodoro && snap.state != TimerState::Idle {
        let mut map = load_pomo_states(db);
        map.insert(
            routine_id,
            ResumeState {
                remaining_secs: snap.remaining_secs,
                phase: snap.phase,
                pomodoro_index: snap.pomodoro_index,
            },
        );
        save_pomo_states(db, &map)?;
    }
    Ok(())
}

/// Drop any suspended state for a routine (after it resumes or is stopped).
fn clear_pomo(db: &rusqlite::Connection, routine_id: i64) -> Result<(), String> {
    let mut map = load_pomo_states(db);
    if map.remove(&routine_id).is_some() {
        save_pomo_states(db, &map)?;
    }
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
        resume: None,
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
        let cur = s.engine.snapshot();
        // Early-return guard: target is already the current, still-active routine.
        if cur.state != TimerState::Idle && cur.routine_id == Some(routine_id) {
            return Ok(cur);
        }
        // Suspend a running pomodoro so it can resume later, then stop+persist.
        if cur.state != TimerState::Idle {
            if let Some(rid) = cur.routine_id {
                suspend_pomo(&s.db, rid, &cur)?;
            }
            if let Some(done) = s.engine.stop() {
                crate::db::sessions::insert(&s.db, &done).map_err(|e| e.to_string())?;
            }
        }
        let routine = crate::db::routines::get(&s.db, routine_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("routine {} not found", routine_id))?;
        // Recompute already_done AFTER the stop+insert so fresh seconds are included
        let tz = *chrono::Local::now().offset();
        let day = crate::core::stats::day_of(chrono::Utc::now(), tz);
        let sessions = crate::db::sessions::all(&s.db).map_err(|e| e.to_string())?;
        let already_done = crate::core::stats::seconds_per_routine(&sessions, day, tz)
            .get(&routine_id)
            .copied()
            .unwrap_or(0);
        // routine already complete for today (continuous) — don't start a zero-length session
        if !routine.pomodoro_enabled && already_done >= routine.target_seconds {
            return Ok(s.engine.snapshot());
        }
        // Resume a previously suspended pomodoro block, if one exists for this routine.
        let resume = if routine.pomodoro_enabled {
            load_pomo_states(&s.db).get(&routine_id).cloned()
        } else {
            None
        };
        let mut cfg = build_config(&routine, already_done);
        cfg.resume = resume;
        s.engine.start(cfg);
        s.current_routine_name = Some(routine.name.clone());
        clear_pomo(&s.db, routine_id)?;
        s.engine.snapshot()
    }; // guard dropped
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(snap)
}

#[tauri::command]
pub fn timer_pause(state: State<'_, Mutex<AppState>>, app: AppHandle) -> Result<TimerSnapshot, String> {
    let snap = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.engine.pause();
        s.engine.snapshot()
    }; // guard dropped
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(snap)
}

#[tauri::command]
pub fn timer_resume(state: State<'_, Mutex<AppState>>, app: AppHandle) -> Result<TimerSnapshot, String> {
    let snap = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.engine.resume();
        s.engine.snapshot()
    }; // guard dropped
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(snap)
}

#[tauri::command]
pub fn timer_skip_break(state: State<'_, Mutex<AppState>>, app: AppHandle) -> Result<TimerSnapshot, String> {
    let snap = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.engine.skip_break();
        s.engine.snapshot()
    }; // guard dropped
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(snap)
}

#[tauri::command]
pub fn timer_stop(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let snap = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        // Capture the routine id BEFORE stop() resets it, so we can clear its
        // suspended state — an explicit 세션 종료 means "start fresh next time".
        let rid = s.engine.snapshot().routine_id;
        if let Some(done) = s.engine.stop() {
            crate::db::sessions::insert(&s.db, &done).map_err(|e| e.to_string())?;
        }
        if let Some(id) = rid {
            clear_pomo(&s.db, id)?;
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
        // Suspend a running pomodoro so it can resume later, then stop+persist.
        let cur = s.engine.snapshot();
        if let Some(rid) = cur.routine_id {
            suspend_pomo(&s.db, rid, &cur)?;
        }
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
        // routine already complete for today (continuous) — don't start a zero-length session
        if !routine.pomodoro_enabled && already_done >= routine.target_seconds {
            return Ok(s.engine.snapshot());
        }
        // Resume a previously suspended pomodoro block, if one exists for this routine.
        let resume = if routine.pomodoro_enabled {
            load_pomo_states(&s.db).get(&routine_id).cloned()
        } else {
            None
        };
        let mut cfg = build_config(&routine, already_done);
        cfg.resume = resume;
        s.engine.start(cfg);
        s.current_routine_name = Some(routine.name.clone());
        clear_pomo(&s.db, routine_id)?;
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

// ── Stats + Settings commands ──────────────────────────────────────────────

#[tauri::command]
pub fn stats_today(state: State<'_, Mutex<AppState>>) -> Result<TodayStats, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let routines = crate::db::routines::list(&s.db).map_err(|e| e.to_string())?;
    let sessions = crate::db::sessions::all(&s.db).map_err(|e| e.to_string())?;
    let rule_str = crate::db::settings::streak_rule(&s.db).map_err(|e| e.to_string())?;
    let rule = match rule_str.as_str() {
        "any_completed" => StreakRule::AnyCompleted,
        "all_completed" => StreakRule::AllCompleted,
        _ => StreakRule::Focused,
    };
    let tz = *chrono::Local::now().offset();
    let today = stats::day_of(chrono::Utc::now(), tz);
    Ok(today_stats(&routines, &sessions, rule, today, tz))
}

#[tauri::command]
pub fn settings_get(state: State<'_, Mutex<AppState>>) -> Result<HashMap<String, String>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    map.insert("theme".to_string(), crate::db::settings::theme(&s.db).map_err(|e| e.to_string())?);
    map.insert("streak_rule".to_string(), crate::db::settings::streak_rule(&s.db).map_err(|e| e.to_string())?);
    Ok(map)
}

#[tauri::command]
pub fn settings_set(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    {
        let s = state.lock().map_err(|e| e.to_string())?;
        crate::db::settings::set(&s.db, &key, &value).map_err(|e| e.to_string())?;
    } // guard dropped
    app.emit("settings://changed", ()).map_err(|e| e.to_string())?;
    Ok(())
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

#[cfg(test)]
mod stats_tests {
    use super::today_stats;
    use crate::core::model::*;
    use chrono::{Utc, TimeZone, FixedOffset};
    fn utc() -> FixedOffset { FixedOffset::east_opt(0).unwrap() }
    fn routine(id: i64, target: i64) -> Routine {
        Routine { id, name: "r".into(), icon: "x".into(), color: None, target_seconds: target,
            pomodoro_enabled: true, focus_minutes: 25, break_minutes: 5, sort_order: id, archived: false, created_at: "".into() }
    }
    fn sess(routine_id: i64, ts: i64, secs: i64) -> FocusSession {
        let t = Utc.timestamp_opt(ts,0).unwrap();
        FocusSession { id: 0, routine_id, started_at: t, ended_at: t, seconds: secs, completed: false }
    }
    #[test]
    fn aggregates_today() {
        let base = 1_700_000_000;
        let day = crate::core::stats::day_of(Utc.timestamp_opt(base,0).unwrap(), utc());
        let routines = vec![routine(1, 800)];
        let sessions = vec![sess(1, base, 800)];
        let st = today_stats(&routines, &sessions, StreakRule::Focused, day, utc());
        assert_eq!(st.total_secs, 800);
        assert_eq!(st.completed, 1);
        assert_eq!(st.routine_count, 1);
        assert_eq!(st.remaining_secs, 0);
        assert_eq!(st.streak, 1);
        assert_eq!(st.best_streak, 1);
        assert_eq!(st.per_routine.get(&1), Some(&800));
    }
}
