use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use crate::core::timer::{TimerEngine, TimerState, TimerEvent};

pub struct AppState {
    pub engine: TimerEngine,
    pub db: rusqlite::Connection,
    pub current_routine_name: Option<String>, // for the tray title "딥워크 24:13"
}

pub fn spawn_tick(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            // One lock: advance, drain+persist any auto-finalized session, capture tray name.
            let (snap, name, persisted) = {
                let state = app.state::<Mutex<AppState>>();
                let mut s = state.lock().unwrap();
                let snap = s.engine.tick();
                let mut persisted = false;
                if let Some(done) = s.engine.take_completed() {
                    let _ = crate::db::sessions::insert(&s.db, &done);
                    s.current_routine_name = None;
                    persisted = true;
                }
                let name = s.current_routine_name.clone();
                (snap, name, persisted)
            }; // guard dropped before any emit/await
            let _ = app.emit("timer://tick", &snap);
            if snap.state_changed { let _ = app.emit("timer://state", &snap); }
            if persisted { let _ = app.emit("routines://changed", ()); }
            if let Some(tray) = app.tray_by_id("main-tray") {
                let title = if snap.state == TimerState::Idle {
                    None
                } else {
                    Some(match name {
                        Some(n) => format!("{} {}", n, snap.remaining_label),
                        None => snap.remaining_label.clone(),
                    })
                };
                let _ = tray.set_title(title);
            }
            if let Some(ev) = snap.event {
                let (title, body) = match ev {
                    TimerEvent::FocusEnded => ("집중 완료", "휴식 시간이에요."),
                    TimerEvent::BreakEnded => ("휴식 끝", "다시 집중해볼까요?"),
                    TimerEvent::TargetReached => ("목표 달성", "오늘 목표를 채웠어요!"),
                };
                let _ = app.notification().builder().title(title).body(body).sound("default").show();
            }
        }
    });
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
