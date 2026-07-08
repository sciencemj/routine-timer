# iOS/iPadOS 포트 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 기존 macOS 데스크톱 앱(Tauri v2 + Svelte 5)에 iOS/iPadOS 타겟을 추가해, 백그라운드에서도 정확히 동작하는 자기 기기용 v1을 만든다.

**Architecture:** 코어(`core/`)·DB·프론트엔드는 재사용. 데스크톱 전용 셸(트레이·팝오버·afplay)은 `#[cfg(desktop)]`로 분리. iOS 백그라운드 정지 문제는 (a) 시작·상태변경 시 목표까지의 모든 phase 경계를 로컬 알림으로 미리 예약하고, (b) 포그라운드 복귀 시 경과 초만큼 엔진을 fast-forward(`timer_resync`)해 화면을 보정하는 두 조각으로 해결한다.

**Tech Stack:** Rust / Tauri v2.11 / `tauri-plugin-notification` 2.3.3 / `time` 0.3 / Svelte 5 runes / bun / vitest / cargo test.

## Global Constraints

- 배포 범위: 시뮬레이터 + 무료 서명 자기 기기. App Store 아님.
- 유니버설 단일 빌드 (iPhone + iPad).
- 커밋 규약: Conventional Commits. 커밋 메시지 끝에 `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- 브랜치: `feat/ios-ipados-port` (이미 생성됨, spec 커밋 있음).
- 기존 게이트 항상 green 유지: `cd src-tauri && cargo test`, `cargo build`(0 warning), `bun run check`(0/0), `bun run test`, `bun run build`.
- `core/timer.rs`의 `tick()`은 순수 카운터로 **유지**(테스트 결정성). 벽시계 diff를 tick 안에 넣지 않는다.
- 모든 모바일 전용 코드(알림 예약·권한·mobile 모듈)는 `#[cfg(mobile)]`, macOS 셸은 `#[cfg(desktop)]`, afplay는 `#[cfg(target_os = "macos")]`로 게이트. 데스크톱 빌드 동작은 이 포트로 바뀌면 안 된다.
- 번들 ID: `com.minjun.dailyroutinetimer` 재사용.
- 예약 알림 체인 상한: 48개 경계 또는 24시간 지평.

## File Structure

- `src-tauri/src/core/timer.rs` — `TimerEngine::future_boundaries()` 추가 (순수, 테스트 대상).
- `src-tauri/src/state.rs` — `AppState.last_tick_at` 추가, `spawn_tick`에서 매 틱 갱신, 즉시 알림/afplay 게이트.
- `src-tauri/src/commands.rs` — `timer_resync`·`is_mobile` 커맨드 추가, 각 timer 커맨드에 `#[cfg(mobile)]` reschedule 훅.
- `src-tauri/src/mobile.rs` — **신규** `#[cfg(mobile)]` 모듈: `reschedule()`, 이벤트 문구, 권한.
- `src-tauri/src/lib.rs` — 데스크톱 셸(`setup_desktop`) 분리, 모바일 권한 요청, 신규 커맨드 등록, `run`/`on_window_event` 게이트.
- `src-tauri/Cargo.toml` — `time` 의존성 추가, `tauri-plugin-positioner`를 desktop 타겟 전용으로 이동.
- `src/lib/commands.ts` — `timerResync`·`isMobile` 래퍼.
- `src/App.svelte` — 마운트에서 `is_mobile`→`.mobile` 클래스, `visibilitychange`→`timerResync`; `.mobile` CSS(safe-area, 패딩 제거).
- `src-tauri/gen/apple/` — `tauri ios init` 산출물(커밋).

---

### Task 1: `future_boundaries` 엔진 메서드 (미래 phase 경계 열거)

예약 알림이 실제 `tick()`이 이벤트를 쏘는 시점과 정확히 일치하도록, 현재 Running 상태에서 목표를 채울 때까지의 (오프셋초, 이벤트) 목록을 계산하는 순수 메서드. 테스트로 `tick()` 반복 결과와 동일함을 증명한다.

**Files:**
- Modify: `src-tauri/src/core/timer.rs` (impl block에 메서드 추가; `#[cfg(test)] mod tests`에 테스트 추가)

**Interfaces:**
- Consumes: 기존 `TimerEngine` 비공개 필드(`state`, `phase`, `remaining`, `mode`, `focus_secs`, `break_secs`, `target_secs`, `already_done_secs`, `session_focus_secs`), `TimerEvent`, `Phase`, `Mode`, `TimerConfig`.
- Produces: `pub fn future_boundaries(&self, cap: usize) -> Vec<(i64, TimerEvent)>` — Task 3의 `mobile::reschedule`가 사용.

- [ ] **Step 1: 실패하는 테스트 작성**

`src-tauri/src/core/timer.rs`의 `#[cfg(test)] mod tests` 안에 추가 (파일 하단 tests 모듈에; `use super::*;`가 이미 있다고 가정, 없으면 추가):

```rust
    // future_boundaries가 실제 tick() 이벤트 시퀀스/오프셋과 일치해야 한다.
    fn drive_tick_boundaries(mut eng: TimerEngine) -> Vec<(i64, TimerEvent)> {
        let mut out = Vec::new();
        let mut t = 0i64;
        // 최대 24h 안전 상한.
        while eng.state() == TimerState::Running && t < 24 * 3600 {
            let snap = eng.tick();
            t += 1;
            if let Some(ev) = snap.event {
                out.push((t, ev));
            }
        }
        out
    }

    fn pomo_engine() -> TimerEngine {
        let mut eng = TimerEngine::new(Box::new(crate::core::clock::SystemClock));
        eng.start(TimerConfig {
            routine_id: 1,
            mode: Mode::Pomodoro,
            focus_secs: 1500, // 25m
            break_secs: 300,  // 5m
            target_secs: 3900, // 65m -> 3 focus blocks (25+25+15)
            already_done_secs: 0,
            resume: None,
        });
        eng
    }

    #[test]
    fn future_boundaries_matches_pomodoro_tick_sequence() {
        let predicted = pomo_engine().future_boundaries(48);
        let actual = drive_tick_boundaries(pomo_engine());
        assert_eq!(predicted, actual);
        assert_eq!(
            predicted,
            vec![
                (1500, TimerEvent::FocusEnded),
                (1800, TimerEvent::BreakEnded),
                (3300, TimerEvent::FocusEnded),
                (3600, TimerEvent::BreakEnded),
                (4500, TimerEvent::TargetReached),
            ]
        );
    }

    #[test]
    fn future_boundaries_continuous_is_single_target() {
        let mut eng = TimerEngine::new(Box::new(crate::core::clock::SystemClock));
        eng.start(TimerConfig {
            routine_id: 1, mode: Mode::Continuous,
            focus_secs: 1500, break_secs: 300, target_secs: 600,
            already_done_secs: 0, resume: None,
        });
        assert_eq!(eng.future_boundaries(48), vec![(600, TimerEvent::TargetReached)]);
    }

    #[test]
    fn future_boundaries_empty_when_not_running() {
        let eng = TimerEngine::new(Box::new(crate::core::clock::SystemClock));
        assert!(eng.future_boundaries(48).is_empty());
    }

    #[test]
    fn ticking_past_target_finalizes_completed_session() {
        // timer_resync는 이 tick 반복을 재사용한다: gap초만큼 tick하면 완료
        // 세션이 나오고 상태가 Idle이 된다.
        let mut eng = TimerEngine::new(Box::new(crate::core::clock::SystemClock));
        eng.start(TimerConfig {
            routine_id: 7, mode: Mode::Continuous,
            focus_secs: 1500, break_secs: 300, target_secs: 5,
            already_done_secs: 0, resume: None,
        });
        for _ in 0..5 { let _ = eng.tick(); }
        assert_eq!(eng.state(), TimerState::Idle);
        let done = eng.take_completed().expect("완료 세션");
        assert_eq!(done.seconds, 5);
        assert!(done.completed);
    }
```

`TimerEvent`가 `PartialEq`를 derive하는지 확인. 현재 `#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]`이므로 `assert_eq!` 가능. `Vec<(i64, TimerEvent)>` 비교 OK.

- [ ] **Step 2: 테스트 실패 확인**

Run: `cd src-tauri && cargo test future_boundaries`
Expected: FAIL — `no method named `future_boundaries` found`.

- [ ] **Step 3: 메서드 구현**

`src-tauri/src/core/timer.rs`의 `impl TimerEngine { ... }` 안(예: `snapshot()` 근처)에 추가:

```rust
    /// 현재 Running 상태에서 목표를 채울 때까지의 미래 phase 경계를
    /// `(지금부터 오프셋 초, 이벤트)`로 열거한다. Running이 아니면 빈 벡터.
    /// `tick()`의 전이 규칙(target_filled 우선, 그다음 remaining==0)을 그대로
    /// 모사하므로, 예약한 iOS 로컬 알림이 라이브 엔진과 같은 시점에 발화한다.
    /// `cap`개 경계 또는 24시간에서 멈춘다(무제한 target_secs==0 pomodoro 방지).
    pub fn future_boundaries(&self, cap: usize) -> Vec<(i64, TimerEvent)> {
        let mut out = Vec::new();
        if self.state != TimerState::Running {
            return out;
        }
        let mut offset: i64 = 0;
        let mut phase = self.phase;
        let mut remaining = self.remaining;
        let mut done = self.already_done_secs + self.session_focus_secs;
        while out.len() < cap {
            match phase {
                Phase::Focus => {
                    let to_target = self.target_secs - done;
                    // target_filled은 focus tick마다 decrement 후 검사되므로,
                    // 이 focus 블록 안에서 목표가 먼저 차면 TargetReached로 끝난다.
                    if self.target_secs > 0 && to_target <= remaining {
                        offset += to_target.max(0);
                        out.push((offset, TimerEvent::TargetReached));
                        break;
                    }
                    // focus 블록이 끝남.
                    offset += remaining;
                    done += remaining;
                    if self.mode == Mode::Pomodoro {
                        out.push((offset, TimerEvent::FocusEnded));
                        phase = Phase::Break;
                        remaining = self.break_secs;
                    } else {
                        // Continuous인데 target도 없음(target==0) — 경계 없음.
                        break;
                    }
                }
                Phase::Break => {
                    offset += remaining;
                    out.push((offset, TimerEvent::BreakEnded));
                    phase = Phase::Focus;
                    remaining = self.focus_secs;
                }
            }
            if offset > 24 * 3600 {
                break;
            }
        }
        out
    }
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cd src-tauri && cargo test future_boundaries && cargo test ticking_past_target`
Expected: PASS (future_boundaries 3개 + ticking_past_target 1개).

- [ ] **Step 5: 전체 코어 테스트 + 빌드 확인**

Run: `cd src-tauri && cargo test && cargo build`
Expected: 전부 PASS, 0 warning.

- [ ] **Step 6: 커밋**

```bash
cd /Users/sciencemj/Desktop/Rust/todo_timer
git add src-tauri/src/core/timer.rs
git commit -m "feat(timer): future_boundaries — 미래 phase 경계 열거 (예약 알림용)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: `timer_resync` — 복귀 시 경과 보정 + `last_tick_at`

포그라운드 복귀 시 얼어붙은 동안의 경과 초만큼 엔진을 fast-forward해 화면을 보정하는 커맨드. `tick()`을 재사용하므로 위상 전환·target finalize가 그대로 처리된다.

**Files:**
- Modify: `src-tauri/src/state.rs:7-11` (`AppState`에 `last_tick_at` 필드), `src-tauri/src/state.rs:18-30` (틱마다 갱신), `src-tauri/src/state.rs:76-80` (테스트 생성자)
- Modify: `src-tauri/src/commands.rs` (`timer_resync` 커맨드 추가)
- Modify: `src-tauri/src/lib.rs` (AppState 생성자에 필드, `invoke_handler`에 `timer_resync` 등록)

**Interfaces:**
- Consumes: `TimerEngine::tick()`, `TimerEngine::take_completed()`, `TimerEngine::snapshot()`, `crate::db::sessions::insert`.
- Produces: `#[tauri::command] pub fn timer_resync(...) -> Result<TimerSnapshot, String>` (JS `timer_resync`), `AppState.last_tick_at: DateTime<Utc>`.

- [ ] **Step 1: `AppState`에 `last_tick_at` 필드 추가**

`src-tauri/src/state.rs`, `use` 구역에 chrono 추가하고 struct 수정:

```rust
use chrono::{DateTime, Utc};
```

```rust
pub struct AppState {
    pub engine: TimerEngine,
    pub db: rusqlite::Connection,
    pub current_routine_name: Option<String>, // for the tray title "딥워크 24:13"
    /// 마지막으로 tick 루프가 엔진을 전진시킨 벽시계 시각. iOS 백그라운드 복귀 시
    /// (now - last_tick_at)만큼 fast-forward해 화면을 보정한다(timer_resync).
    pub last_tick_at: DateTime<Utc>,
}
```

- [ ] **Step 2: `spawn_tick` 루프에서 매 틱 갱신**

`src-tauri/src/state.rs`의 잠금 블록 안(`let snap = s.engine.tick();` 다음 줄)에 추가:

```rust
                let snap = s.engine.tick();
                s.last_tick_at = Utc::now();
```

- [ ] **Step 3: state.rs 테스트 생성자에 필드 추가**

`src-tauri/src/state.rs`의 `#[test] fn appstate_can_create_and_list_routines` 안 `AppState { ... }` 리터럴에 추가:

```rust
        let mut st = AppState {
            engine: TimerEngine::new(Box::new(SystemClock)),
            db,
            current_routine_name: None,
            last_tick_at: Utc::now(),
        };
```

- [ ] **Step 4: `timer_resync` 커맨드 추가**

`src-tauri/src/commands.rs`의 `timer_get_state` 다음에 추가:

```rust
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
```

주의: Task 3 전에는 `crate::mobile`이 없으므로 이 `#[cfg(mobile)]` 줄은 **데스크톱 빌드에서 컴파일 제외**되어 문제없다(데스크톱에서 `cargo build`/`cargo test`로 검증). 모바일 빌드는 Task 3 이후 Task 6에서 처음 수행한다.

- [ ] **Step 5: lib.rs AppState 생성자 + 커맨드 등록**

`src-tauri/src/lib.rs`의 `setup` 안 `AppState` 생성:

```rust
            app.manage(Mutex::new(AppState {
                engine: TimerEngine::new(Box::new(SystemClock)),
                db: conn,
                current_routine_name: None,
                last_tick_at: chrono::Utc::now(),
            }));
```

`invoke_handler!` 목록에 `commands::timer_get_state,` 다음 줄로 추가:

```rust
            commands::timer_resync,
```

- [ ] **Step 6: 빌드·테스트 확인**

Run: `cd src-tauri && cargo test && cargo build`
Expected: PASS, 0 warning.

- [ ] **Step 7: 커밋**

```bash
cd /Users/sciencemj/Desktop/Rust/todo_timer
git add src-tauri/src/state.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(timer): timer_resync — 백그라운드 복귀 경과 보정 + last_tick_at

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: `mobile.rs` — 예약 알림 체인 + 권한

모바일 전용 모듈. Running 상태의 미래 경계 전부를 로컬 알림으로 절대 시각에 예약하고, 비Running이면 전부 취소한다. 상태를 바꾸는 각 timer 커맨드가 이를 호출한다.

**Files:**
- Create: `src-tauri/src/mobile.rs`
- Modify: `src-tauri/Cargo.toml` (`time` 의존성)
- Modify: `src-tauri/src/lib.rs` (`#[cfg(mobile)] mod mobile;`, 모바일 권한 요청)
- Modify: `src-tauri/src/commands.rs` (각 timer 커맨드 emit 직후 `#[cfg(mobile)]` reschedule 호출)

**Interfaces:**
- Consumes: `TimerEngine::future_boundaries` (Task 1), `crate::state::AppState`, `TimerEvent`, `tauri_plugin_notification::{NotificationExt, Schedule}`, `time::OffsetDateTime`.
- Produces: `#[cfg(mobile)] pub fn reschedule<R: tauri::Runtime>(app: &tauri::AppHandle<R>, state: &std::sync::Mutex<crate::state::AppState>)`.

- [ ] **Step 1: `time` 의존성 추가**

`src-tauri/Cargo.toml`의 `[dependencies]`에 추가(플러그인의 `Schedule::At { date: time::OffsetDateTime }` 구성용):

```toml
time = "0.3"
```

- [ ] **Step 2: `mobile.rs` 작성**

`src-tauri/src/mobile.rs` 생성:

```rust
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
```

- [ ] **Step 3: lib.rs에 모듈 선언 + 모바일 권한 요청**

`src-tauri/src/lib.rs` 상단 모듈 선언부에 추가:

```rust
#[cfg(mobile)]
pub mod mobile;
```

`setup` 안, `spawn_tick(...)` 호출 다음에 모바일 권한 요청 추가:

```rust
            crate::state::spawn_tick(app.handle().clone());

            #[cfg(mobile)]
            {
                use tauri_plugin_notification::NotificationExt;
                let _ = app.notification().request_permission();
            }
```

- [ ] **Step 4: 각 timer 커맨드에 reschedule 훅 추가**

`src-tauri/src/commands.rs`의 아래 커맨드들에서, 마지막 `app.emit("timer://state", &snap)...?;` 다음 줄에 `#[cfg(mobile)] crate::mobile::reschedule(&app, state.inner());`를 넣는다. `timer_resync`는 Task 2에서 이미 넣었다.

`timer_start` (line ~393):

```rust
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    #[cfg(mobile)]
    crate::mobile::reschedule(&app, state.inner());
    Ok(snap)
```

`timer_pause`, `timer_resume`, `timer_skip_break`도 동일하게 각자의 `app.emit("timer://state", &snap)...?;` 다음에 두 줄 추가.

`timer_stop` (line ~448, `Ok(())` 앞):

```rust
    app.emit("timer://state", &snap).map_err(|e| e.to_string())?;
    #[cfg(mobile)]
    crate::mobile::reschedule(&app, state.inner());
    Ok(())
```

`timer_switch` (line ~473) 및 `db_reset` (line ~541)의 마지막 `timer://state` emit 다음에도 동일하게 추가.

주의: `timer_start`/`timer_switch`의 `if !started { return Ok(snap); }` 조기 반환 경로는 상태가 안 바뀌므로 reschedule 불필요 — 그대로 둔다.

- [ ] **Step 5: 데스크톱 빌드·테스트 확인**

Run: `cd src-tauri && cargo test && cargo build`
Expected: PASS, 0 warning. (`#[cfg(mobile)]` 코드는 데스크톱에서 제외되므로 `mobile.rs`·reschedule 호출은 컴파일되지 않는다. `time` 의존성만 추가됨.)

- [ ] **Step 6: 커밋**

```bash
cd /Users/sciencemj/Desktop/Rust/todo_timer
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/mobile.rs src-tauri/src/lib.rs src-tauri/src/commands.rs
git commit -m "feat(mobile): 예약 알림 체인 — 목표까지 모든 phase 경계 로컬 알림 예약

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: 플랫폼 분기 — 데스크톱 셸 `#[cfg(desktop)]`, `is_mobile` 커맨드

트레이·팝오버·positioner·close-to-tray·Dock Reopen을 데스크톱 전용으로 분리하고, afplay를 macOS 전용으로, 즉시 알림을 desktop 전용으로 게이트한다. positioner 의존성도 desktop 타겟으로 옮겨 iOS 컴파일을 깨끗하게 한다.

**Files:**
- Modify: `src-tauri/Cargo.toml` (`tauri-plugin-positioner`를 desktop 타겟 전용으로)
- Modify: `src-tauri/src/lib.rs` (desktop `use` 게이트, `setup_desktop` 분리, `on_window_event`/`run` 게이트, `is_mobile` 등록)
- Modify: `src-tauri/src/state.rs:48-60` (즉시 알림 `#[cfg(desktop)]`, afplay `#[cfg(target_os="macos")]`)
- Modify: `src-tauri/src/commands.rs` (`is_mobile` 커맨드)

**Interfaces:**
- Produces: `#[tauri::command] pub fn is_mobile() -> bool` (JS `is_mobile`).

- [ ] **Step 1: positioner를 desktop 타겟 의존성으로 이동**

`src-tauri/Cargo.toml`의 `[dependencies]`에서 `tauri-plugin-positioner` 줄을 제거하고, 파일 하단에 타겟별 섹션 추가:

```toml
[target.'cfg(desktop)'.dependencies]
tauri-plugin-positioner = { version = "2.3.2", features = ["tray-icon"] }
```

- [ ] **Step 2: `is_mobile` 커맨드 추가**

`src-tauri/src/commands.rs`의 `settings_set` 근처(Stats+Settings 구역)나 파일 끝 커맨드 자리에 추가:

```rust
/// 프론트가 모바일 레이아웃(safe-area, 데스크톱 크롬 제거)을 켜기 위한 플랫폼
/// 감지. 뷰포트 폭 기반 CSS는 iPad를 데스크톱으로 오판하므로 Rust가 알려준다.
#[tauri::command]
pub fn is_mobile() -> bool {
    cfg!(mobile)
}
```

- [ ] **Step 3: state.rs — 전역 use 제거, tray-title/즉시알림/afplay 게이트**

세 곳을 바꾼다. 트레이·알림·afplay는 전부 데스크톱 전용이므로 모바일 tick 루프는 순수하게 엔진 전진 + 이벤트 emit만 한다.

(a) 파일 상단 `use tauri_plugin_notification::NotificationExt;` (state.rs:4) **줄을 삭제**한다. (아래 desktop 블록 안에서 지역 `use`로 다시 가져오므로 전역 use는 모바일에서 미사용 경고가 된다.)

(b) tray 타이틀 갱신 블록 `if let Some(tray) = app.tray_by_id("main-tray") { ... }` 전체를 `#[cfg(desktop)]`로 감싼다. (모바일은 `tray-icon` feature가 없어 `tray_by_id`/`set_title`이 컴파일되지 않을 수 있으므로 필수):

```rust
            #[cfg(desktop)]
            if let Some(tray) = app.tray_by_id("main-tray") {
                // macOS does NOT clear the menubar title when passed None — it keeps
                // the last text. Pass an empty string so the countdown disappears
                // (leaving just the icon) once the timer goes Idle.
                let title: Option<String> = if snap.state == TimerState::Idle {
                    Some(String::new())
                } else {
                    Some(match name {
                        Some(n) => format!("{} {}", n, snap.remaining_label),
                        None => snap.remaining_label.clone(),
                    })
                };
                let _ = tray.set_title(title);
            }
```

(c) `if let Some(ev) = snap.event { ... }` 블록을 아래로 교체(즉시 알림은 desktop 전용, afplay는 macOS 전용; 모바일은 예약 알림 체인이 담당):

```rust
            if let Some(ev) = snap.event {
                #[cfg(desktop)]
                {
                    let (title, body) = match ev {
                        TimerEvent::FocusEnded => ("집중 완료", "휴식 시간이에요."),
                        TimerEvent::BreakEnded => ("휴식 끝", "다시 집중해볼까요?"),
                        TimerEvent::TargetReached => ("목표 달성", "오늘 목표를 채웠어요!"),
                    };
                    // 데스크톱: 즉시 알림. 모바일: mobile::reschedule가 미리 예약한
                    // 로컬 알림이 포그라운드/백그라운드 모두에서 발화한다(이중 방지).
                    use tauri_plugin_notification::NotificationExt;
                    let _ = app.notification().builder().title(title).body(body).sound("default").show();
                    // macOS만: Focus/DND로 알림이 묵음이어도 확실히 소리를 낸다.
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("afplay")
                        .arg("/System/Library/Sounds/Glass.aiff")
                        .spawn();
                }
                #[cfg(mobile)]
                let _ = ev; // 모바일: 예약 알림이 담당, 여기선 no-op
            }
```

(검증: `cargo build`가 0 warning이어야 한다.)

- [ ] **Step 4: lib.rs 데스크톱 셸 분리**

`src-tauri/src/lib.rs`:

(a) 상단 import 정리. 현재 `use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};`(lib.rs:6)를 아래로 교체하고, 그 아래 tray/menu/positioner use들에 `#[cfg(desktop)]`을 붙인다(`Manager`는 양쪽에서 쓰이므로 게이트 안 함):

```rust
use tauri::Manager;
#[cfg(desktop)]
use tauri::{WebviewUrl, WebviewWindowBuilder};
#[cfg(desktop)]
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
#[cfg(desktop)]
use tauri::menu::{MenuBuilder, MenuItemBuilder};
#[cfg(desktop)]
use tauri_plugin_positioner::{WindowExt, Position};
```

(b) tray + popover + positioner 설정을 별도 함수로 추출한다. `pub fn run()` 정의 위에 아래 함수를 추가한다. **본문**은 현재 `.setup(|app| { ... })` 안에서 `app.handle().plugin(tauri_plugin_positioner::init())?;` 줄부터 popover `WebviewWindowBuilder::new(...) ... .build()?;` 문(끝의 `?;` 포함)까지의 **연속된 블록을 글자 그대로 잘라내어** 함수 본문에 붙여 넣는다(내용 변경 없음 — 그 코드는 이미 `app`을 쓰고, 여기서도 `app: &App`이라 동일하게 컴파일된다). 즉 `open_item`/`pause_item`/`quit_item`/`menu`/`_tray`/`_popover` 정의 전부가 이 함수로 이동한다:

```rust
#[cfg(desktop)]
fn setup_desktop(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // <<< 여기에 setup에서 잘라낸 positioner-init ~ _popover.build()? 블록을 그대로 붙인다 >>>
    Ok(())
}
```

이동 후, `.setup(...)` 안에는 (c)의 공유 코드만 남는다.

(c) `.setup(|app| { ... })` 안에서 공유 부분만 남기고 데스크톱 부분 호출:

```rust
        .setup(|app| {
            let dir = app.path().app_data_dir().expect("no app data dir");
            std::fs::create_dir_all(&dir).ok();
            let conn = crate::db::open(dir.join("routine.db").to_str().unwrap())?;
            crate::db::migrate(&conn)?;
            app.manage(Mutex::new(AppState {
                engine: TimerEngine::new(Box::new(SystemClock)),
                db: conn,
                current_routine_name: None,
                last_tick_at: chrono::Utc::now(),
            }));
            crate::state::spawn_tick(app.handle().clone());

            #[cfg(mobile)]
            {
                use tauri_plugin_notification::NotificationExt;
                let _ = app.notification().request_permission();
            }

            #[cfg(desktop)]
            setup_desktop(app)?;

            Ok(())
        })
```

(d) `on_window_event` 클로저 본문을 `#[cfg(desktop)]`로 감싼다(모바일엔 트레이·팝오버 창이 없음):

```rust
        .on_window_event(|window, event| {
            #[cfg(desktop)]
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
            #[cfg(mobile)]
            let _ = (window, event);
        })
```

(e) `.run(|app, event| { ... Reopen ... })`의 본문을 `#[cfg(desktop)]`로 감싼다:

```rust
        .run(|app, event| {
            #[cfg(desktop)]
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            #[cfg(mobile)]
            let _ = (app, event);
        });
```

(f) `invoke_handler!`에 `is_mobile` 등록(`db_reset,` 다음):

```rust
            commands::is_mobile,
```

- [ ] **Step 5: 데스크톱 빌드·테스트·동작 확인**

Run: `cd src-tauri && cargo build && cargo test`
Expected: PASS, **0 warning**. (데스크톱 cfg는 기존 코드를 전부 포함하므로 데스크톱 동작 불변.)

Run(수동, 선택): `bun run tauri dev` — 트레이 카운트다운·팝오버·close-to-tray가 이전과 동일하게 동작하는지 확인.

- [ ] **Step 6: 커밋**

```bash
cd /Users/sciencemj/Desktop/Rust/todo_timer
git add src-tauri/Cargo.toml src-tauri/src/lib.rs src-tauri/src/state.rs src-tauri/src/commands.rs
git commit -m "refactor(platform): 데스크톱 셸 cfg(desktop) 분리 + is_mobile 커맨드

트레이/팝오버/positioner/close-to-tray/Reopen을 desktop 전용으로,
afplay를 macOS 전용, 즉시 알림을 desktop 전용으로 게이트. 데스크톱 동작 불변.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: 프론트엔드 — 플랫폼 클래스 · resync · 반응형 CSS

`is_mobile`로 `.mobile` 클래스를 켜고, 복귀 시 `timer_resync`를 호출하고, safe-area·데스크톱 패딩 제거 CSS를 추가한다. 데스크톱 동작(클래스 없음)은 불변.

**Files:**
- Modify: `src/lib/commands.ts` (`timerResync`·`isMobile` 래퍼)
- Modify: `src/App.svelte` (마운트 효과 + `.mobile` CSS)

**Interfaces:**
- Consumes: `commands.timerResync()`, `commands.isMobile()`.

- [ ] **Step 1: commands.ts 래퍼 추가**

`src/lib/commands.ts`의 `commands` 객체에 추가(`dbReset` 다음):

```ts
  timerResync: () => invoke<TimerSnapshot>('timer_resync'),
  isMobile: () => invoke<boolean>('is_mobile'),
```

- [ ] **Step 2: App.svelte 마운트 효과 — 플랫폼 클래스 + visibilitychange**

`src/App.svelte`의 `<script>`에서 `commands` import 확인(없으면 `import { commands } from '$lib/commands';` 추가). 기존 isMain `$effect`를 아래로 교체:

```ts
  $effect(() => {
    if (!isMain) return;
    let alive = true;
    let cleanup: (() => void) | undefined;
    themeStore.init();
    initRoutinesListeners().then((fn) => { if (alive) cleanup = fn; else fn(); });

    // 모바일(iPhone/iPad)이면 .mobile 클래스로 safe-area·데스크톱 크롬 제거를 켠다.
    commands.isMobile().then((m) => {
      if (alive && m) document.documentElement.classList.add('mobile');
    });

    // 포그라운드 복귀 시 백그라운드 경과를 엔진에 반영(화면 보정).
    const onVisible = () => {
      if (document.visibilityState === 'visible') commands.timerResync();
    };
    document.addEventListener('visibilitychange', onVisible);

    return () => {
      alive = false;
      cleanup?.();
      document.removeEventListener('visibilitychange', onVisible);
    };
  });
```

(데스크톱: `isMobile()`가 false → `.mobile` 미부착. `visibilitychange`는 데스크톱에서 `timer_resync`를 호출하지만 gap≈0이라 무해.)

- [ ] **Step 3: `.mobile` CSS 추가**

`src/App.svelte`의 `<style>` 끝에 추가:

```css
  /* iOS/iPadOS: 노치·홈 인디케이터 여백 + 데스크톱 traffic-light 패딩 제거. */
  :global(html.mobile) .top-bar {
    padding-left: max(16px, env(safe-area-inset-left));
    padding-right: max(16px, env(safe-area-inset-right));
    padding-top: env(safe-area-inset-top);
    height: calc(44px + env(safe-area-inset-top));
  }
  :global(html.mobile) .content {
    padding-bottom: env(safe-area-inset-bottom);
  }
```

`top-bar`의 `data-tauri-drag-region`은 모바일에서 드래그 대상이 없어 무해하므로 그대로 둔다.

- [ ] **Step 4: 게이트 확인**

Run:
```bash
cd /Users/sciencemj/Desktop/Rust/todo_timer
bun run check && bun run test && bun run build
```
Expected: `check` 0 errors / 0 warnings, `test` 통과, `build` 성공.

- [ ] **Step 5: 커밋**

```bash
cd /Users/sciencemj/Desktop/Rust/todo_timer
git add src/lib/commands.ts src/App.svelte
git commit -m "feat(mobile): 플랫폼 클래스 + 복귀 resync + safe-area CSS

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Tauri iOS 프로젝트 생성 + 시뮬레이터/실기기 검증

Xcode 프로젝트를 스캐폴드하고 시뮬레이터에서 부팅, 백그라운드 알림·복귀 보정·레이아웃을 수동 검증한다. 환경 의존(Xcode 필요).

**Files:**
- Create: `src-tauri/gen/apple/**` (`tauri ios init` 산출물)
- Modify(필요 시): `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`(타겟별 feature 조정)

- [ ] **Step 1: 사전 요건 확인**

Run: `xcodebuild -version && xcrun simctl list devices available | head`
Expected: Xcode 버전 출력 + 사용 가능한 시뮬레이터 목록. (없으면 App Store에서 Xcode 설치 후 `xcode-select --install`.)

- [ ] **Step 2: iOS 프로젝트 초기화**

Run: `cd /Users/sciencemj/Desktop/Rust/todo_timer && bun run tauri ios init`
Expected: `src-tauri/gen/apple/` 생성, 성공 메시지.

만약 `tauri` 또는 `tauri-plugin-positioner`의 feature가 iOS 타겟 컴파일을 깨면(예: `macos-private-api`, `tray-icon` 관련 링크 에러), `src-tauri/Cargo.toml`에서 해당 feature를 desktop 타겟 전용으로 분리한다:

```toml
# 공통 tauri는 mobile-safe feature만
tauri = { version = "2.11.4", features = ["image-png"] }

[target.'cfg(desktop)'.dependencies]
tauri = { version = "2.11.4", features = ["tray-icon", "macos-private-api"] }
tauri-plugin-positioner = { version = "2.3.2", features = ["tray-icon"] }
```

(positioner는 Task 4에서 이미 desktop 전용. tauri feature 분리는 init 에러가 실제로 날 때만 적용.)

- [ ] **Step 3: 시뮬레이터 부팅**

Run: `cd /Users/sciencemj/Desktop/Rust/todo_timer && bun run tauri ios dev`
Expected: iOS 시뮬레이터가 뜨고 앱이 로드됨. 첫 실행 시 알림 권한 프롬프트가 뜨는지 확인(→ 허용).

- [ ] **Step 4: 수동 검증 체크리스트**

- [ ] 루틴 하나를 포모도로(25/5 또는 테스트용 짧은 값)로 시작.
- [ ] 홈 버튼(시뮬레이터: Device ▸ Home)으로 백그라운드 전환 → focus 종료 시각에 "집중 완료" 알림, break 종료 시각에 "휴식 끝" 알림이 뜨는지.
- [ ] 앱 복귀 시 남은 시간이 실제 경과를 반영(백그라운드 동안 줄어든 만큼 보정)하는지.
- [ ] iPhone·iPad 시뮬레이터 각각에서 상단바가 노치와 겹치지 않고, traffic-light용 좌측 여백이 없는지.
- [ ] 원형 타이머·탭바가 화면 폭에 맞는지(FocusView `max-width:360px`).

- [ ] **Step 5: (선택) 실기기 무료 서명 빌드**

Xcode로 `src-tauri/gen/apple/*.xcodeproj`를 열어 Signing & Capabilities에서 개인 Apple ID 팀 선택 후 기기 타겟으로 Run. 또는 `bun run tauri ios build --debug`. (무료 서명 앱은 7일 후 재설치 필요.)

- [ ] **Step 6: 커밋**

```bash
cd /Users/sciencemj/Desktop/Rust/todo_timer
git add src-tauri/gen src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "chore(ios): tauri ios init — Xcode 프로젝트 스캐폴드

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## 검증 요약 (플랜 완료 기준)

- `cd src-tauri && cargo test`: `future_boundaries_*` 3개 포함 전부 PASS.
- `cd src-tauri && cargo build`: 0 warning (데스크톱·모바일 양 타겟).
- `bun run check`: 0/0. `bun run test`·`bun run build`: 통과.
- 데스크톱 앱: 트레이·팝오버·알림·afplay 동작 이전과 동일(회귀 없음).
- iOS 시뮬레이터: 포모도로 백그라운드 경계 알림 발화 + 복귀 화면 보정 + safe-area 레이아웃.

## 문서화된 수용 한계

- 예약 체인 상한 48개/24h: 초장기 세션·`target_secs==0` 무제한 루틴은 이후 경계 알림 누락 가능.
- resync gap 24h 클램프: 24시간 넘게 백그라운드였던 세션은 그 이상 전진 안 함(현실적 focus 세션엔 무해).
- App Store 미배포: 무료 서명은 7일마다 재설치.
