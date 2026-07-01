use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use crate::core::model::{NewRoutine, Routine};
use crate::state::AppState;

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
    let s = state.lock().map_err(|e| e.to_string())?;
    let created_at = chrono::Utc::now().to_rfc3339();
    let routine = crate::db::routines::create(&s.db, &new, &created_at)
        .map_err(|e| e.to_string())?;
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    Ok(routine)
}

#[tauri::command]
pub fn routine_update(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    routine: Routine,
) -> Result<(), String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    crate::db::routines::update(&s.db, &routine).map_err(|e| e.to_string())?;
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn routine_delete(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    id: i64,
) -> Result<(), String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    crate::db::routines::set_archived(&s.db, id, true).map_err(|e| e.to_string())?;
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn routine_reorder(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    ordered_ids: Vec<i64>,
) -> Result<(), String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    crate::db::routines::reorder(&s.db, &ordered_ids).map_err(|e| e.to_string())?;
    app.emit("routines://changed", ()).map_err(|e| e.to_string())?;
    Ok(())
}
