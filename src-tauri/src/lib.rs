pub mod commands;
pub mod core;
pub mod db;
pub mod state;

use std::sync::Mutex;
use tauri::Manager;
use crate::core::clock::SystemClock;
use crate::core::timer::TimerEngine;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&dir).ok();
            let conn = crate::db::open(dir.join("routine.db").to_str().unwrap())?;
            crate::db::migrate(&conn)?;
            app.manage(Mutex::new(AppState {
                engine: TimerEngine::new(Box::new(SystemClock)),
                db: conn,
                current_routine_name: None,
            }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::routines_list,
            commands::routine_create,
            commands::routine_update,
            commands::routine_delete,
            commands::routine_reorder,
            commands::timer_start,
            commands::timer_pause,
            commands::timer_resume,
            commands::timer_stop,
            commands::timer_skip_break,
            commands::timer_switch,
            commands::timer_get_state,
            commands::stats_today,
            commands::settings_get,
            commands::settings_set,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
