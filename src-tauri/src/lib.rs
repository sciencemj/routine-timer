pub mod commands;
pub mod core;
pub mod db;
pub mod state;

use std::sync::Mutex;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri_plugin_positioner::{WindowExt, Position};
use crate::core::clock::SystemClock;
use crate::core::timer::TimerEngine;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .on_window_event(|window, event| {
            match window.label() {
                "main" => {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
                "popover" => {
                    if let tauri::WindowEvent::Focused(false) = event { let _ = window.hide(); }
                }
                _ => {}
            }
        })
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
            crate::state::spawn_tick(app.handle().clone());

            app.handle().plugin(tauri_plugin_positioner::init())?;

            let open_item = MenuItemBuilder::with_id("open", "대시보드 열기").build(app)?;
            let pause_item = MenuItemBuilder::with_id("pause", "일시정지 / 계속").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "종료").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&open_item, &pause_item, &quit_item]).build()?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .title("--:--")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "pause" => {
                        let state = app.state::<std::sync::Mutex<AppState>>();
                        let mut s = state.lock().unwrap();
                        match s.engine.state() {
                            crate::core::timer::TimerState::Paused => s.engine.resume(),
                            crate::core::timer::TimerState::Idle => {}
                            _ => s.engine.pause(),
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("popover") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.move_window(Position::TrayBottomCenter);
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            let _popover = WebviewWindowBuilder::new(app, "popover", WebviewUrl::App("index.html#/popover".into()))
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .resizable(false)
                .transparent(true)
                .inner_size(400.0, 560.0)
                .title("")
                .build()?;

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
            commands::stats_report,
            commands::settings_get,
            commands::settings_set,
            commands::db_reset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
