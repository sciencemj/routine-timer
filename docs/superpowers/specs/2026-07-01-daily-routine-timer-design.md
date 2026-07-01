# Daily Routine Timer — Design Spec

**Date:** 2026-07-01
**Status:** Approved (design), pre-implementation
**Author:** minjun park (design) · brainstormed with Claude

## 1. 개요 (Overview)

하루 루틴을 타이머로 관리하는 데스크탑 앱. 핵심 은유: **"루틴별 요구 시간을 원형 타이머로 채운다"** — 각 루틴에 하루 목표 시간(요구 시간)이 있고, 집중 세션으로 그 링을 채워 완료한다.

macOS 메뉴바에 실시간 카운트다운을 띄우고, 클릭하면 현황 팝오버가 뜬다.

## 2. 범위 (Scope)

### MVP (이번 스펙 — 구현 대상)
- 대시보드 (오늘의 루틴, 진행 링, 요약, streak)
- 루틴 CRUD + 설정 (아이콘·이름·요구 시간·포모도로 설정)
- 포커스 모드 (포모도로 사이클 + 연속 카운트다운)
- macOS 메뉴바 위젯 (트레이 카운트다운 + 웹뷰 팝오버)
- 테마 (라이트 / 다크 / 시스템)

### 다음 슬라이스 (스키마·구조는 지원하되 지금 구현 X)
- 리포트 / GitHub-잔디 스타일 히트맵 (`focus_session` 로그로 근거는 확보)
- iOS 타깃 (Rust 코어 + Svelte UI 재사용; 트레이/팝오버는 macOS 전용)

## 3. 스택 결정 (Stack Decisions)

| 항목 | 선택 | 이유 |
|------|------|------|
| 셸 | Tauri v2 | macOS 지금, iOS 확장 경로 확보 |
| 타깃 (MVP) | macOS | 메뉴바 위젯이 우선순위이자 macOS 전용 |
| 프론트엔드 | Svelte + Vite | 경량·반응형, Tauri 궁합, 메인·팝오버 컴포넌트 공유 |
| 코어/상태 | Rust | 타이머 엔진이 창과 독립적으로 tick → 트레이 갱신 필수 |
| 영속화 | SQLite (`rusqlite`, 번들 기능) | 세션 로그 → 리포트/히트맵 근거 |
| 트레이 위치잡기 | `tauri-plugin-positioner` | 팝오버를 트레이 아래 배치 |
| 알림 | `tauri-plugin-notification` | 세션/휴식 종료 알림 |

## 4. 프로세스 / 창 구조 (Architecture)

단일 Tauri 앱. **Rust `TimerEngine`이 단일 진실원(single source of truth).** 백그라운드 tick(tokio `interval`, 1초)로 진행하며 상태를 관리·영속화하고 이벤트를 emit한다.

- **메인 창** — Svelte SPA. 라우트: `/`(대시보드) · `/focus` · `/report`(다음 슬라이스) · `/settings`.
  - 닫기(빨간 버튼) = 종료가 아니라 트레이 상주(hide). 실제 종료는 트레이 메뉴 "종료".
- **팝오버 창** — 같은 Svelte 앱의 `/popover` 라우트. borderless + always-on-top + skip-taskbar, 기본 숨김.
  - 트레이 클릭 시 트레이 아이콘 아래 위치(`tauri-plugin-positioner`)로 토글. 포커스 잃으면 숨김.
- **트레이 아이콘** — `TrayIcon::set_title`로 상태 텍스트 갱신:
  - 집중 중: `딥워크 24:13`
  - 유휴: `남은 5:40` (오늘 남은 총 요구 시간 요약) 또는 아이콘만.
- **Dock 아이콘 유지** — 정식 앱(메뉴바 전용 액세서리 아님).

### 이벤트 / 커맨드 모델
- Rust → 프론트 (emit): `timer://tick`(매초 remaining), `timer://state`(상태 전이), `routines://changed`, `settings://changed`.
- 프론트 → Rust (`#[tauri::command]`):
  - 루틴: `routines_list`, `routine_create`, `routine_update`, `routine_delete`, `routine_reorder`
  - 타이머: `timer_start(routine_id)`, `timer_pause`, `timer_resume`, `timer_stop`, `timer_skip_break`, `timer_switch(routine_id)`, `timer_get_state`
  - 통계: `stats_today`, `stats_range(from,to)`(다음 슬라이스 리포트용)
  - 설정: `settings_get`, `settings_set(key,value)`
  - 팝오버/창: `popover_toggle`, `focus_window_open`

## 5. 데이터 모델 (SQLite)

```sql
CREATE TABLE routine (
  id              INTEGER PRIMARY KEY,
  name            TEXT    NOT NULL,
  icon            TEXT    NOT NULL,        -- emoji or icon key
  color           TEXT,                    -- accent hex, nullable
  target_seconds  INTEGER NOT NULL,        -- 하루 요구 시간
  pomodoro_enabled INTEGER NOT NULL DEFAULT 1,
  focus_minutes   INTEGER NOT NULL DEFAULT 25,
  break_minutes   INTEGER NOT NULL DEFAULT 5,
  sort_order      INTEGER NOT NULL,
  archived        INTEGER NOT NULL DEFAULT 0,
  created_at      TEXT    NOT NULL         -- ISO8601
);

CREATE TABLE focus_session (
  id          INTEGER PRIMARY KEY,
  routine_id  INTEGER NOT NULL REFERENCES routine(id),
  started_at  TEXT    NOT NULL,            -- ISO8601 (local)
  ended_at    TEXT    NOT NULL,
  seconds     INTEGER NOT NULL,            -- 순수 집중 초 (휴식·일시정지 제외)
  completed   INTEGER NOT NULL             -- 계획대로 종료 vs 중단
);
CREATE INDEX idx_session_started ON focus_session(started_at);

CREATE TABLE app_settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
-- settings keys: theme ('system'|'light'|'dark'),
--                streak_rule ('focused'|'any_completed'|'all_completed')
```

### 파생 계산 (derived, 저장하지 않음)
- **오늘 루틴 진행** = 오늘(로컬 자정 기준) 해당 루틴 `focus_session.seconds` 합.
- **루틴 완료** = 진행 합 ≥ `target_seconds`. "5개 중 2개 완료"·"남은 2:30" 근거.
- **하루 집중 총합** = 오늘 전체 `seconds` 합.
- **streak(연속 N일)** — `streak_rule`로 규칙 선택 (기본 `focused`):
  - `focused`: 하루 ≥1 집중 세션 있는 날 연속 (기본).
  - `any_completed`: 하루 ≥1 루틴이 목표 달성한 날 연속.
  - `all_completed`: 그날 모든 활성 루틴이 목표 달성한 날 연속.

## 6. 타이머 엔진 (Rust 상태머신 — TDD 대상)

```rust
enum TimerState { Idle, Running, Paused, Break }

struct ActiveSession {
    routine_id:     i64,
    mode:           Mode,          // Pomodoro | Continuous
    pomodoro_index: u32,           // "포모도로 2"
    phase:          Phase,         // Focus | Break
    remaining:      Duration,      // 현재 페이즈 남은 시간
    accumulated:    Duration,      // 이번 세션 순수 집중 누적
    started_at:     DateTime,
}
```

### 전이 (transitions)
- `timer_start(routine)`: Idle → Running. 루틴의 `pomodoro_enabled`로 mode 결정.
  - Pomodoro: `remaining = focus_minutes`.
  - Continuous: `remaining = target_seconds − 오늘 이미 채운 초` (목표까지 연속 카운트다운).
- tick(1s): `remaining -= 1`, `accumulated += 1`(Focus phase일 때만), 트레이 갱신 + `timer://tick` emit.
- Pomodoro Focus phase 끝(remaining=0): → Break(`remaining = break_minutes`), 알림, `pomodoro_index += 1`. Break 끝: → 다음 Focus.
- Continuous remaining=0(목표 달성): → Idle, 완료 알림, 세션 기록.
- Pomodoro 모드는 목표 도달 시 **자동 종료하지 않고** 사용자가 종료할 때까지 focus/break 사이클을 계속한다. 루틴 "완료" 표시는 파생 합(≥target)으로 별도 판정(§5).
- `timer_pause`/`resume`: Running ↔ Paused (tick 정지, accumulated 보존).
- `timer_stop`: → Idle. `focus_session`에 기록(seconds = accumulated, completed = 목표달성 여부).
- `timer_skip_break`: Break → 즉시 다음 Focus.
- `timer_switch(routine)`: 현재 세션 stop-기록 후 새 루틴 start.

## 7. 화면 (Screens)

시각 소스 = claude_design 프로젝트 (아래 §11). 아래는 각 화면의 목적·핵심 요소.

- **대시보드 `/`**
  - 시간대 인사(좋은 아침이에요 / 좋은 오후예요), 날짜·시계.
  - 오늘의 루틴 카드 그리드: 아이콘 + 이름 + 원형 진행 링(오늘 채운 / 요구 시간) + 상태(미시작/진행 중/완료).
  - 요약 바: "남은 2시간 30분 · 5개 루틴 중 2개 완료".
  - 하루 집중 총합, streak(연속 N일 · 최고 M일).
  - "새 루틴" 추가 진입.
- **루틴 설정 (모달 또는 `/settings` 내)**: 아이콘(emoji 선택), 이름, 요구 시간, 포모도로 on/off, 집중 분·휴식 분, 정렬/삭제.
- **포커스 `/focus`**
  - 큰 원형 카운트다운(JetBrains Mono 숫자), "남은 시간 · 25:00".
  - 포모도로 표시: "집중 중 · 포모도로 2", "25분 집중 · 5분 휴식".
  - 컨트롤: 일시정지/계속하기, 세션 종료, 뒤로.
  - 휴식 화면(세션 종료 후 휴식 5분), "다음 제안"(다음 루틴 제안).
- **팝오버 `/popover`**: 현황(남은 · 완료 수) + 현재 루틴 + 빠른 집중 시작/일시정지. 클릭 시 같은 테마 포커스로 진입.
- **리포트 `/report`** (다음 슬라이스): 13주 잔디 히트맵, 최근 7일 바, KPI 타일(이번 주 집중 · 일 평균 · 지난주 대비).

## 8. 메뉴바 / 트레이 (macOS)

- 트레이 아이콘 + `set_title` 카운트다운(§4).
- 좌클릭 → 팝오버 창 토글, `tauri-plugin-positioner`로 트레이 기준 배치. 포커스 상실 시 숨김.
- 트레이 우클릭(또는 팝오버 내) 메뉴: 대시보드 열기, 일시정지/종료, 앱 종료.
- 팝오버는 메인 앱과 같은 Svelte 코드/테마 공유.

## 9. 테마 (Theme)

- 값: 라이트 / 다크 / 시스템. `app_settings.theme` 저장.
- Tauri에서 시스템 테마 감지(`window.theme()`) + 변경 구독 → 시스템 모드일 때 자동 반영.
- 대시보드·포커스·팝오버 **동일 테마 적용**(디자인 요구).
- 방향성: 라이트 = 밝은 배경 + 은은한 그림자, 다크 = 어두운 배경 + 블루 글로우 포인트. 숫자 타이머는 JetBrains Mono, 본문은 Pretendard.

## 10. 테스트 전략

- **Rust (TDD)**: `TimerEngine` 전이(포모도로 사이클, 연속 카운트다운, 완료 판정, pause/resume/switch), 통계 집계(일 합계·완료 수·streak 3규칙), SQLite repo CRUD. Tauri 창 의존성 없이 순수 로직으로 분리해 테스트.
- **프론트 (Vitest)**: 시간 포맷(`mm:ss`, `Hh Mm`), 링 진행 계산, 테마 스토어, 이벤트 구독 → 뷰 반영.
- **수동**: 트레이 카운트다운 갱신, 팝오버 위치·포커스 동작, 시스템 테마 전환.

## 11. 디자인 소스 (Visual Source of Truth)

- claude_design 프로젝트: `Daily routine timer app` (id `93331616-a080-4e2c-921e-f87139874321`, owner minjun park).
- 파일: `Daily Routine Timer.dc.html`(인터랙티브 프로토타입: 대시보드↔포커스, 라이트/다크), `Routine Timer.dc.html`(레이아웃/리포트 방향 다수), `uploads/pasted-1782827741317-0.png`(참조 이미지).
- 접근: `DesignSync` MCP `get_file` (읽기 전용). 구현 시 화면별 정확한 토큰(색·간격·폰트)은 여기서 추출.
- 토큰 힌트(캔버스 크롬 기준): bg `#e7e6e3`, text `#16181d`, accent blue `#2f6bed`; fonts Pretendard Variable / JetBrains Mono / Space Grotesk. (앱 화면 실팔레트는 구현 시 확정.)

## 12. 결정 로그 (Decisions)

- MVP = macOS 우선(메뉴바 포함). iOS·리포트 다음 슬라이스.
- Svelte + Vite / Rust 코어 / SQLite / 웹뷰 팝오버(네이티브 메뉴 아님 — 디자인 충실).
- streak 기본 `focused`, 설정에서 규칙 변경 가능.
- 메인 창 닫기 = 트레이 상주. Dock 아이콘 유지.

## 13. 미해결 / 구현 중 확정할 항목

- 루틴 아이콘: emoji 선택기로 시작(MVP). 커스텀 아이콘셋은 추후.
- 하루 경계: 로컬 자정 기준(타임존 처리 명시).
- 포모도로 토글 위치: 루틴별 기본값 + 포커스 화면에서 세션 단위 override 허용 여부(구현 시 확정).
- 팝오버 정확한 크기·애니메이션은 디자인 추출 후.
