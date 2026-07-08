# iOS/iPadOS 포트 설계 (v1, 자기 기기용)

작성일: 2026-07-08
상태: 승인됨 (구현 대기)

## 목표와 스코프

기존 macOS 데스크톱 앱(Tauri v2 + Svelte 5)에 iOS/iPadOS 타겟을 추가한다.

- **배포 범위**: 시뮬레이터 + 무료 서명으로 본인 iPhone/iPad에 설치. **App Store 배포 아님** (유료 개발자 계정·프로비저닝·심사 불필요).
- **바이너리**: Tauri v2 iOS는 유니버설 — iPhone과 iPad가 단일 빌드.
- **재사용**: `core/`(timer·stats·model·clock), `db/`(rusqlite bundled), `commands.rs` invoke 핸들러, Svelte 프론트엔드는 그대로 포팅. 데스크톱 전용 셸(트레이·팝오버·afplay)만 플랫폼 분기.
- **전제**: Xcode 설치. iOS 최소 버전은 Tauri 기본값(13+).

## 핵심 문제: iOS 백그라운드 정지

`core/timer.rs`의 `tick()`은 **순수 카운터**로, 호출마다 `remaining`을 1 감소시킬 뿐 벽시계 경과를 보지 않는다(테스트 결정성을 위한 의도적 설계, 주석 명시). macOS 데스크톱은 프로세스가 항상 살아 있어 `spawn_tick` tokio 루프가 매초 tick을 호출하므로 문제없다.

iOS는 앱이 백그라운드로 가면 tokio 루프가 정지한다. 결과:
1. 복귀 시 타이머가 얼어붙은 지점부터 재개되어 실제 경과 시간을 못 따라잡는다(화면이 틀림).
2. 백그라운드 동안 phase 종료 이벤트가 발생하지 않아 알람이 안 울린다.

집중 타이머는 사용자가 폰을 내려놓고 딴 일에 집중하는 것이 정상 사용이므로, 이 문제를 반드시 해결한다. 해결은 **(a) 예약 알림**과 **(b) 복귀 시 화면 보정** 두 조각으로 나뉜다.

## 사전 검증 (플러그인 근거)

`tauri-plugin-notification` 2.3.3 소스 직접 확인:

- 예약 API 존재: `NotificationBuilder::schedule(Schedule::At { date: time::OffsetDateTime, repeating, allow_while_idle })` (`src/lib.rs:96`, `src/models.rs:98`).
- 취소/권한: `cancel(Vec<i32>)`, `cancel_all()`, `request_permission()` (`src/mobile.rs`).
- **포그라운드 표시**: `ios/Sources/NotificationHandler.swift:32` `willPresent`가 non-silent 알림에 `[.badge, .sound, .alert]`를 반환 → **예약 알림이 포그라운드에서도 배너+소리로 표시된다.** 따라서 모바일 알람의 단일 소스를 예약 체인으로 삼을 수 있다(즉시 알림과 이중 발생 회피).
- 주의: `date`는 `chrono`가 아닌 `time::OffsetDateTime`. `time` 크레이트를 직접 의존성에 추가하고 `OffsetDateTime::from_unix_timestamp(dt.timestamp())`로 변환한다(이미 트랜지티브로 컴파일됨).
- 한계: iOS는 앱당 대기(pending) 로컬 알림을 **64개**로 제한한다. 예약 체인은 상한을 둔다.

## 설계

### 1. 플랫폼 분기 (`lib.rs`)

`setup`의 macOS 데스크톱 셸을 `#[cfg(desktop)]`로 감싼다:

- 트레이 아이콘(`TrayIconBuilder`, 메뉴, `on_tray_icon_event`),
- 팝오버 `WebviewWindow` + `tauri-plugin-positioner`,
- `on_window_event`(close-to-tray의 `CloseRequested`, 팝오버 `Focused(false)` 숨김),
- `run(...)`의 `RunEvent::Reopen`(Dock 재열기).

데스크톱·모바일 공유: db `open`/`migrate`, `AppState` manage, `spawn_tick`, notification 플러그인 init.

모바일 전용: 첫 실행 시 `notification().request_permission()` 호출(권한 없으면 예약 알림이 안 뜸).

`Cargo.toml`: `tauri`의 `macos-private-api` 피처와 `macOSPrivateApi`/`transparent`는 데스크톱에만 필요하나, 피처는 타겟별 분기가 번거로우므로 유지하되 모바일 빌드에서 팝오버 창을 안 만들면 무해하다. (transparent 창을 안 생성하므로 실제 영향 없음.)

### 2. 사운드/트레이 (`state.rs` tick 루프)

- `tray_by_id("main-tray")`는 모바일에서 트레이를 안 만들었으므로 `None` → 기존 `if let Some(tray)` 블록이 자동 no-op. **변경 불필요.**
- tick 루프의 즉시 알림(`notification().builder()...show()`)과 afplay `std::process::Command` 둘 다 `#[cfg(target_os = "macos")]`로 감싼다. 모바일 알람은 예약 체인(아래 3a)이 담당하므로 즉시 알림을 빼서 예약 알림과의 이중 발생을 막는다.

### 3. 백그라운드 정확성

#### (a) 예약 알림 체인

새 모듈 `mobile.rs` (`#[cfg(mobile)]`), 함수 `reschedule(app, &snap)`:

- `snap.state == Running`이면 현재 상태로부터 **목표를 채울 때까지의 미래 경계를 전부 열거**하여, 각 경계를 절대 시각(`now + offset`)으로 `Schedule::At` 예약한다. 알림 ID는 고정 범위(예: 1000+index).
  - 예약 전 `cancel_all()`로 이전 체인 제거 후 재예약.
  - 각 경계 본문은 이벤트별: `FocusEnded`="집중 완료 / 휴식 시간이에요.", `BreakEnded`="휴식 끝 / 다시 집중해볼까요?", `TargetReached`="목표 달성 / 오늘 목표를 채웠어요!" (기존 macOS 문구 재사용).
  - 상한: 최대 48개 경계 또는 24시간 지평 중 먼저 도달. `target_secs == 0`(무제한) 루틴은 이 상한에서 잘린다 — 문서화된 수용 한계.
- `snap.state != Running`(Paused/Idle)이면 `cancel_all()`.
- 호출 지점: 상태를 바꾸는 각 커맨드(`timer_start`/`timer_pause`/`timer_resume`/`timer_skip_break`/`timer_switch`/`timer_stop`)의 스냅샷 emit 직후. 데스크톱에서는 `#[cfg(mobile)]`로 컴파일 제외.

경계 열거는 `core/timer.rs`에 **순수 함수**로 구현한다:

```
fn future_boundaries(mode, phase, remaining, focus_secs, break_secs,
                     target_secs, done_secs, cap) -> Vec<(offset_secs, TimerEvent)>
```

- `done_secs = already_done + session_focus`(누적 집중 초).
- Continuous: 단일 경계 `(remaining, TargetReached)`.
- Pomodoro: Focus 블록이 목표를 채우면 `TargetReached`로 종료, 아니면 `FocusEnded` 후 Break, Break 끝나면 `BreakEnded` 후 다음 Focus… `cap` 도달까지 반복.
- 이 함수는 `tick()`의 전이 규칙(`target_filled` 우선, 그다음 `remaining==0`)과 일치해야 하며, 단위 테스트로 검증한다(같은 입력에서 tick 반복 결과의 이벤트 시퀀스와 offset이 일치).

#### (b) 복귀 시 화면 보정 (resync)

- `AppState`에 `last_tick_at: DateTime<Utc>` 추가. `spawn_tick` 루프가 매 틱마다 갱신.
- 새 커맨드 `timer_resync`:
  - `gap = (now - last_tick_at).num_seconds()`.
  - `engine.tick()`을 `gap`회 반복 호출해 얼어붙은 동안의 경과를 따라잡는다(위상 전환·target finalize 로직 재사용). 반복 중 완료 세션이 나오면 `take_completed()` → `db::sessions::insert`로 persist.
  - 반복 중 개별 이벤트로 알림을 발생시키지 **않는다**(알람은 이미 예약 체인이 백그라운드에서 울렸음). 마지막에 스냅샷 1회만 emit.
  - `last_tick_at = now`.
- 프론트: `FocusView`(또는 `App`)의 마운트 `$effect`에서 `visibilitychange` 리스너 등록 → `document.visibilityState === 'visible'`가 되면 `invoke('timer_resync')`. alive-flag 정리 패턴 준수.
- `tick()`은 순수 카운터로 **유지**(기존 Rust 테스트 불변). resync는 tick을 여러 번 부를 뿐 tick 자체를 안 바꾼다.
- resync는 미래 경계 시각을 바꾸지 않으므로 예약 재설정이 불필요하다(이미 지난 경계는 백그라운드에서 발화됨, 미래 경계는 절대시각 그대로 유효).

### 4. 최소 반응형 UI

- 플랫폼 감지: 새 커맨드 `is_mobile() -> bool { cfg!(mobile) }`. 프론트 마운트에서 `invoke('is_mobile')` → `document.documentElement.classList.toggle('mobile', v)`. (뷰포트 폭 기반 CSS는 iPad를 데스크톱으로 오판하므로 쓰지 않는다.)
- `.mobile` 하에서:
  - `App.svelte` top-bar의 traffic-light용 `padding: 0 84px` 제거, `data-tauri-drag-region` 무효(모바일엔 드래그 대상 없음).
  - `env(safe-area-inset-top/bottom/left/right)`로 노치·홈 인디케이터 여백 확보.
- `FocusView`의 원형 타이머가 iPhone 폭(~390pt)에서 넘치지 않는지 확인, 필요 시 `min(vw, ...)`/`max-width`로 조정.
- 팝오버 라우트(`/popover`)는 모바일에서 창을 안 만들어 도달 불가 → 그대로 둔다.

### 5. Tauri iOS 프로젝트

- `tauri ios init`으로 `src-tauri/gen/apple/` Xcode 프로젝트 생성(커밋 대상).
- 개발: `tauri ios dev`(시뮬레이터). 실기기: 무료 서명으로 `tauri ios build`(7일 유효 서명).
- `tauri.conf.json`에 필요한 경우 iOS 관련 설정 추가(번들 ID `com.minjun.dailyroutinetimer` 재사용).

## 검증 기준

- 기존 게이트 유지: `cargo test`, `cargo build`(0 warning), `bun run check`(0/0), `bun run test`, `bun run build` 모두 green.
- 신규 Rust 단위 테스트:
  - `future_boundaries`가 `tick()` 반복의 이벤트 시퀀스/오프셋과 일치(Continuous 단일 경계, Pomodoro 25/5 체인, target 도달로 마지막이 `TargetReached`, 상한 clamp).
  - resync fast-forward가 gap초 후 올바른 phase/remaining/완료 세션을 만든다(주입 Clock 사용).
- 수동 검증(시뮬레이터/실기기):
  - 포모도로 시작 → 홈으로 나가 백그라운드 → 25분 후 "집중 완료" 알림, 30분 후 "휴식 끝" 알림 발화.
  - 복귀 시 화면 remaining이 실제 경과 반영(보정).
  - iPhone·iPad 레이아웃 safe-area 정상, traffic-light 패딩 없음.

## 문서화된 수용 한계

- 예약 체인 상한 48개/24h: 초장기 세션 또는 `target_secs == 0` 루틴은 이후 경계 알림이 누락될 수 있음. 자기용 v1 허용.
- App Store 미배포: 실기기 무료 서명은 7일마다 재설치 필요.
