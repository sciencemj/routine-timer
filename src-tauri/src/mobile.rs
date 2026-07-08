//! iOS/iPadOS 전용 로직. 데스크톱 빌드에는 컴파일되지 않는다(`#[cfg(mobile)]`).
//!
//! macOS는 프로세스가 항상 살아 tick 루프가 매초 알림 시점을 직접 처리하지만,
//! iOS는 백그라운드에서 앱이 정지한다. 그래서 Running으로 진입/변경될 때마다
//! 목표를 채울 때까지의 모든 phase 경계를 로컬 알림으로 미리 예약해 둔다.

use std::sync::Mutex;
use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::{NotificationExt, Schedule};

use crate::core::timer::TimerEvent;
use crate::state::AppState;

/// 예약 알림 체인 상한(iOS 앱당 대기 알림 64개 한도 아래로).
const MAX_BOUNDARIES: usize = 48;

/// 알림 ID 베이스(다른 알림과 겹치지 않도록). id = BASE + index.
const ID_BASE: i32 = 1000;

fn event_text(ev: TimerEvent) -> (&'static str, &'static str) {
    match ev {
        TimerEvent::FocusEnded => ("집중 완료", "휴식 시간이에요."),
        TimerEvent::BreakEnded => ("휴식 끝", "다시 집중해볼까요?"),
        TimerEvent::TargetReached => ("목표 달성", "오늘 목표를 채웠어요!"),
    }
}

/// 현재 엔진 상태에 맞춰 예약 알림을 재설정한다. Running이면 미래 경계 전부를
/// 절대 시각으로 예약하고, 아니면 전부 취소한다. 상태를 바꾸는 커맨드의 emit
/// 직후(잠금 해제 상태)에서 호출한다 — 여기서 잠깐 다시 잠근다.
pub fn reschedule<R: Runtime>(app: &AppHandle<R>, state: &Mutex<AppState>) {
    let boundaries = {
        let s = match state.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        s.engine.future_boundaries(MAX_BOUNDARIES)
    }; // 잠금 해제 후 알림 API 호출

    let notif = app.notification();
    let _ = notif.cancel_all();

    let now = chrono::Utc::now().timestamp();
    for (i, (offset, ev)) in boundaries.iter().enumerate() {
        let Ok(date) = time::OffsetDateTime::from_unix_timestamp(now + offset) else {
            continue;
        };
        let (title, body) = event_text(*ev);
        let _ = notif
            .builder()
            .id(ID_BASE + i as i32)
            .title(title)
            .body(body)
            .sound("default")
            .schedule(Schedule::At {
                date,
                repeating: false,
                allow_while_idle: true,
            })
            .show();
    }
}
