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

// ── Day-boundary helpers ────────────────────────────────────────────────────
//
// The "day" for stats-bucketing purposes doesn't have to start at midnight —
// users can push it later (e.g. 8 AM) so late-night focus sessions still
// count toward the PREVIOUS day. We implement this without touching
// core::stats at all: shifting the day start to hour H is equivalent to
// bucketing with a FixedOffset that is H hours EARLIER than the real local
// offset (so `date_naive()` doesn't roll over until H:00 local time).

const DEFAULT_DAY_START_HOUR: i64 = 8;

/// Pure helper: shift a real local UTC offset (in seconds) earlier by
/// `day_start_hour` hours, returning the FixedOffset to use for day-bucketing.
/// Never panics — falls back to UTC if the shifted offset is out of range.
pub fn shifted_offset(real_local_secs: i32, day_start_hour: i64) -> FixedOffset {
    let shifted = real_local_secs - (day_start_hour as i32) * 3600;
    FixedOffset::east_opt(shifted).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap())
}

/// Reads the configured day-start hour (default `DEFAULT_DAY_START_HOUR`,
/// clamped to 0..=23) and returns today's date plus the shifted offset to use
/// for all stats bucketing.
fn day_context(db: &rusqlite::Connection) -> (NaiveDate, FixedOffset) {
    let h: i64 = crate::db::settings::get(db, "day_start_hour")
        .ok()
        .flatten()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(DEFAULT_DAY_START_HOUR)
        .clamp(0, 23);
    let real = *chrono::Local::now().offset();
    let tz = shifted_offset(real.local_minus_utc(), h);
    let today = stats::day_of(chrono::Utc::now(), tz);
    (today, tz)
}

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

// ── Report types ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HeatCell {
    pub date: String, // "YYYY-MM-DD"
    pub secs: i64,
    pub level: u8,
}

#[derive(Serialize)]
pub struct ReportData {
    pub heatmap: Vec<HeatCell>, // Sunday-of-13-weeks-ago ..= today, oldest -> newest
    pub this_week_secs: i64,    // this week's Sunday ..= today
    pub last_week_secs: i64,    // previous full week Sun..Sat
    pub daily_avg_secs: i64,    // total over ACTIVE days in the heatmap range / active-day count
    pub month_active_days: i64, // days with >0 focus in the current calendar month (up to today)
    pub streak: u32,
    pub best_streak: u32,
    pub last7: Vec<i64>, // secs for [today-6 ..= today], oldest -> newest
}

/// Pure builder for the Report screen: a ~13-week focus heatmap plus weekly /
/// average KPIs and a 7-day series. Unit-testable without Tauri.
pub fn build_report(
    sessions: &[FocusSession],
    routines: &[Routine],
    rule: StreakRule,
    today: NaiveDate,
    tz: FixedOffset,
) -> ReportData {
    use chrono::Datelike;

    // 0 = Sunday .. 6 = Saturday
    let weekday = today.weekday().num_days_from_sunday() as i64;
    let week_start = today - chrono::Duration::days(weekday); // this week's Sunday
    let heat_start = week_start - chrono::Duration::days(12 * 7); // Sunday 13 weeks ago

    let totals = stats::range_day_totals(sessions, heat_start, today, tz);

    let heatmap: Vec<HeatCell> = totals
        .iter()
        .map(|&(d, secs)| HeatCell {
            date: d.format("%Y-%m-%d").to_string(),
            secs,
            level: stats::heat_level(secs),
        })
        .collect();

    let sum_in = |lo: NaiveDate, hi: NaiveDate| -> i64 {
        totals
            .iter()
            .filter(|&&(d, _)| d >= lo && d <= hi)
            .map(|&(_, s)| s)
            .sum()
    };

    let this_week_secs = sum_in(week_start, today);
    let last_week_start = week_start - chrono::Duration::days(7);
    let last_week_end = week_start - chrono::Duration::days(1);
    let last_week_secs = sum_in(last_week_start, last_week_end);

    // Daily average over ACTIVE (>0) days in the heatmap range.
    let active: Vec<i64> = totals.iter().map(|&(_, s)| s).filter(|&s| s > 0).collect();
    let daily_avg_secs = if active.is_empty() {
        0
    } else {
        active.iter().sum::<i64>() / active.len() as i64
    };

    // Active days in the current calendar month, up to today. `month_start` is
    // always within the heatmap range (a month is far shorter than 13 weeks).
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let month_active_days = totals
        .iter()
        .filter(|&&(d, secs)| secs > 0 && d >= month_start && d <= today)
        .count() as i64;

    let streak = stats::streak(routines, sessions, rule, today, tz);
    let best_streak = stats::max_streak(routines, sessions, rule, today, tz);

    let last7_start = today - chrono::Duration::days(6);
    let last7: Vec<i64> = totals
        .iter()
        .filter(|&&(d, _)| d >= last7_start && d <= today)
        .map(|&(_, s)| s)
        .collect();

    ReportData {
        heatmap,
        this_week_secs,
        last_week_secs,
        daily_avg_secs,
        month_active_days,
        streak,
        best_streak,
        last7,
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

// ── Shared timer-launch helpers (timer_start / timer_switch / timer_stop) ──
//
// Both timer_start and timer_switch end whatever session is currently running
// and then launch a routine from scratch; timer_stop only does the first
// half. Factored out so a fix to the sequence can't drift between callers.

/// Stop the engine (if running) and persist the completed session, if any.
fn stop_and_persist(s: &mut AppState) -> Result<(), String> {
    if let Some(done) = s.engine.stop() {
        crate::db::sessions::insert(&s.db, &done).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Look up `routine_id`, compute today's already-done seconds, and start the
/// engine for it — the launch sequence shared by timer_start and
/// timer_switch. Returns `(snapshot, started)`: when a continuous routine
/// already met its target today, `started` is false and `snapshot` is the
/// CURRENT (unstarted) engine state — callers must return it as-is, without
/// emitting any event.
fn begin_routine(s: &mut AppState, routine_id: i64) -> Result<(TimerSnapshot, bool), String> {
    let routine = crate::db::routines::get(&s.db, routine_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("routine {} not found", routine_id))?;
    // Recompute already_done AFTER the stop+insert so fresh seconds are included
    let (today, tz) = day_context(&s.db);
    let sessions = crate::db::sessions::all(&s.db).map_err(|e| e.to_string())?;
    let already_done = crate::core::stats::seconds_per_routine(&sessions, today, tz)
        .get(&routine_id)
        .copied()
        .unwrap_or(0);
    // routine already complete for today (continuous) — don't start a zero-length session
    if !routine.pomodoro_enabled && already_done >= routine.target_seconds {
        return Ok((s.engine.snapshot(), false));
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
    Ok((s.engine.snapshot(), true))
}

// ── Timer commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn timer_start(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    routine_id: i64,
) -> Result<TimerSnapshot, String> {
    let (snap, started) = {
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
            stop_and_persist(&mut s)?;
        }
        begin_routine(&mut s, routine_id)?
    }; // guard dropped
    if !started {
        return Ok(snap);
    }
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
        stop_and_persist(&mut s)?;
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
    let (snap, started) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        // Suspend a running pomodoro so it can resume later, then stop+persist.
        let cur = s.engine.snapshot();
        if let Some(rid) = cur.routine_id {
            suspend_pomo(&s.db, rid, &cur)?;
        }
        stop_and_persist(&mut s)?;
        // Start the new routine
        begin_routine(&mut s, routine_id)?
    }; // guard dropped
    if !started {
        return Ok(snap);
    }
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(snap)
}

#[tauri::command]
pub fn timer_get_state(state: State<'_, Mutex<AppState>>) -> Result<TimerSnapshot, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.engine.snapshot())
}

/// iOS 포그라운드 복귀용: 앱이 백그라운드에 있는 동안 tick 루프가 멈춰
/// 엔진이 얼어붙는다. 복귀 시 (now - last_tick_at)초만큼 tick()을 반복해
/// 경과를 따라잡고(위상 전환·target finalize 재사용), 완료 세션을 persist한다.
/// 알람은 이미 예약 알림이 백그라운드에서 울렸으므로 여기서 알림은 쏘지 않는다.
#[tauri::command]
pub fn timer_resync(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
) -> Result<TimerSnapshot, String> {
    let (snap, persisted) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now();
        // 음수(시계 역행)면 0, 비정상적으로 크면 24h로 클램프.
        let gap = (now - s.last_tick_at).num_seconds().clamp(0, 24 * 3600);
        s.last_tick_at = now;
        for _ in 0..gap {
            let _ = s.engine.tick();
        }
        let mut persisted = false;
        if let Some(done) = s.engine.take_completed() {
            crate::db::sessions::insert(&s.db, &done).map_err(|e| e.to_string())?;
            s.current_routine_name = None;
            persisted = true;
        }
        (s.engine.snapshot(), persisted)
    }; // guard dropped before emit
    if persisted {
        app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    }
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    // 모바일: 미래 경계 스케줄을 현재 상태에 맞춰 갱신(Task 3에서 정의).
    #[cfg(mobile)]
    crate::mobile::reschedule(&app, state.inner());
    Ok(snap)
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
    let (today, tz) = day_context(&s.db);
    Ok(today_stats(&routines, &sessions, rule, today, tz))
}

#[tauri::command]
pub fn stats_report(state: State<'_, Mutex<AppState>>) -> Result<ReportData, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let routines = crate::db::routines::list(&s.db).map_err(|e| e.to_string())?;
    let sessions = crate::db::sessions::all(&s.db).map_err(|e| e.to_string())?;
    let rule_str = crate::db::settings::streak_rule(&s.db).map_err(|e| e.to_string())?;
    let rule = match rule_str.as_str() {
        "any_completed" => StreakRule::AnyCompleted,
        "all_completed" => StreakRule::AllCompleted,
        _ => StreakRule::Focused,
    };
    let (today, tz) = day_context(&s.db);
    Ok(build_report(&sessions, &routines, rule, today, tz))
}

#[tauri::command]
pub fn settings_get(state: State<'_, Mutex<AppState>>) -> Result<HashMap<String, String>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    map.insert("theme".to_string(), crate::db::settings::theme(&s.db).map_err(|e| e.to_string())?);
    map.insert("streak_rule".to_string(), crate::db::settings::streak_rule(&s.db).map_err(|e| e.to_string())?);
    let day_start_hour = crate::db::settings::get(&s.db, "day_start_hour")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| DEFAULT_DAY_START_HOUR.to_string());
    map.insert("day_start_hour".to_string(), day_start_hour);
    Ok(map)
}

/// "DB 리셋" — discards any running session, wipes routines/sessions/suspended
/// pomodoro state (keeping preferences), and notifies the frontend so both the
/// routines list and the timer UI reset immediately.
#[tauri::command]
pub fn db_reset(state: State<'_, Mutex<AppState>>, app: AppHandle) -> Result<TimerSnapshot, String> {
    let snap = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let _ = s.engine.stop(); // discard any running session (we're wiping)
        crate::db::reset(&s.db).map_err(|e| e.to_string())?;
        s.current_routine_name = None;
        s.engine.snapshot()
    }; // guard dropped
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    Ok(snap)
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
    fn shifted_offset_shifts_day_start_hour_earlier() {
        assert_eq!(shifted_offset(9 * 3600, 8).local_minus_utc(), 1 * 3600);
        assert_eq!(shifted_offset(0, 8).local_minus_utc(), -8 * 3600);
        assert_eq!(shifted_offset(-8 * 3600, 8).local_minus_utc(), -16 * 3600);
    }
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
    use super::{build_report, today_stats};
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

    #[test]
    fn report_shapes_and_totals() {
        // base = 1_700_000_000 == 2023-11-14 (Tuesday) in UTC.
        let base = 1_700_000_000;
        let today = crate::core::stats::day_of(Utc.timestamp_opt(base, 0).unwrap(), utc());
        // today (Tue) 800s, yesterday (Mon, same week) 300s, 10 days ago 600s.
        let sessions = vec![
            sess(1, base, 800),
            sess(1, base - 86_400, 300),
            sess(1, base - 10 * 86_400, 600),
        ];
        let routines = vec![routine(1, 800)];
        let r = build_report(&sessions, &routines, StreakRule::Focused, today, utc());

        // last7: 7 values, oldest -> newest, last is today.
        assert_eq!(r.last7.len(), 7);
        assert_eq!(*r.last7.last().unwrap(), 800);

        // heatmap non-empty and ends on today.
        assert!(!r.heatmap.is_empty());
        assert_eq!(
            r.heatmap.last().unwrap().date,
            today.format("%Y-%m-%d").to_string()
        );

        // this week (Sun..=Tue) = today + yesterday = 1100.
        assert_eq!(r.this_week_secs, 1100);

        // daily avg over 3 active days (800 + 300 + 600) / 3.
        assert_eq!(r.daily_avg_secs, (800 + 300 + 600) / 3);
    }
}
