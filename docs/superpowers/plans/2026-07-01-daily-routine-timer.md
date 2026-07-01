# Daily Routine Timer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a macOS-first Tauri v2 desktop app that manages daily routines by filling each routine's required time with a circular focus timer, with a live menubar countdown + webview popover.

**Architecture:** A pure, Tauri-free Rust core (`TimerEngine` state machine + stats aggregation, both driven by an injectable `Clock`) is the single source of truth. SQLite (rusqlite, bundled) persists routines/sessions/settings. A thin Tauri command+event layer wraps the core; a background 1s tick loop advances the engine, emits `timer://tick`/`timer://state`, updates the tray title, and fires notifications. A plain Svelte 5 SPA (hash-routed) renders the main window (`/`, `/focus`, `/settings`, `/report`) and the borderless menubar popover (`/popover`), both subscribing to the same events.

**Tech Stack:** Tauri v2 (Rust) · Svelte 5 + Vite (TypeScript) · rusqlite · chrono · svelte-spa-router · Vitest · cargo test.

## Global Constraints

Every task's requirements implicitly include this section. Copy version pins verbatim.

- **Platform (this plan):** macOS only. Keep the Dock icon — do **NOT** set `LSUIElement` or `ActivationPolicy::Accessory`.
- **Rust deps (src-tauri/Cargo.toml):**
  - `tauri = { version = "2.11.4", features = ["tray-icon", "image-png"] }`
  - `tauri-build = { version = "2.6.3", features = [] }` (build-dependency)
  - `tauri-plugin-notification = "2.3.3"`
  - `tauri-plugin-positioner = { version = "2.3.2", features = ["tray-icon"] }`
  - `rusqlite = { version = "0.40.1", features = ["bundled"] }`
  - `chrono = { version = "0.4.45", features = ["serde"] }`
  - `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`
  - `tokio = { version = "1", features = ["time"] }`
- **npm deps:** `@tauri-apps/api@2.11.1`, `@tauri-apps/plugin-notification@2.3.3`, `@tauri-apps/plugin-positioner@2.3.2`, `svelte-spa-router@5.1.1`, `pretendard`, `@fontsource/jetbrains-mono`, `@fontsource/space-grotesk`.
- **npm devDeps:** `@tauri-apps/cli@^2.11`, `svelte@5.56.4`, `vite@8.1.2`, `@sveltejs/vite-plugin-svelte@7.1.2`, `typescript@~6.0.3`, `svelte-check@^4.6.0`, `vitest@4.1.9`, `@testing-library/svelte@5.4.2`, `@testing-library/jest-dom@^6`, `jsdom@29.1.1`.
- **Tauri v2 trait split:** `use tauri::{Manager, Emitter};` — `emit`/`emit_to` are on `Emitter`; `manage`/`state`/`get_webview_window`/`tray_by_id` are on `Manager`.
- **Frontend imports:** `invoke` from `@tauri-apps/api/core`; `listen`/`UnlistenFn` from `@tauri-apps/api/event`; `getCurrentWindow` from `@tauri-apps/api/window`.
- **Reactivity:** Svelte 5 runes (`$state`/`$derived`/`$effect`) only work in `.svelte` and `.svelte.ts` files — name reactive stores `*.svelte.ts`.
- **Command naming:** register commands in snake_case (`skip_break`); call `invoke('skip_break', { routineId })` — Tauri maps camelCase arg keys → snake_case params, but the command name is matched verbatim.
- **Event payload keys:** Rust struct field names serialize as-is (snake_case). Frontend payload interfaces use snake_case to match.
- **Design source of truth (visual):** claude_design project `Daily routine timer app` (id `93331616-a080-4e2c-921e-f87139874321`). Files `Daily Routine Timer.dc.html`, `Routine Timer.dc.html`, `uploads/pasted-1782827741317-0.png`. Read via `DesignSync` MCP `get_file`. Extract exact colors/spacing/fonts per screen when building UI (Tasks 24–28). Korean UI copy is authored text — keep verbatim.
- **Deferred (NOT in this plan):** `/report` charts/heatmap (route stub only), iOS.
- **Discipline:** TDD for all pure logic (Rust core, stats, db, frontend stores/utils). DRY. YAGNI. Commit after every green step.

---

## File Structure

**Frontend (`src/`)**
- `main.ts` — entry: import fonts + `app.css`, mount `App.svelte`.
- `App.svelte` — `svelte-spa-router` `<Router>` with routes.
- `app.css` — CSS variables for light/dark theme tokens + font families.
- `lib/time.ts` — `formatDuration`, `formatClock` (pure, tested).
- `lib/types.ts` — shared TS types mirroring Rust payloads.
- `lib/commands.ts` — typed `invoke` wrappers.
- `lib/timer.svelte.ts` — reactive timer store fed by Tauri events (tested).
- `lib/routines.svelte.ts` — routines + today stats store (tested).
- `lib/theme.svelte.ts` — theme store: system detect/follow + override (tested).
- `lib/components/RingTimer.svelte` — circular progress + numerals.
- `lib/components/RoutineCard.svelte` — dashboard routine card.
- `routes/Home.svelte` · `routes/Focus.svelte` · `routes/Settings.svelte` · `routes/Popover.svelte` · `routes/Report.svelte` (stub).
- `test/tauri-mock.ts` — Vitest mock of `@tauri-apps/api` + `emitTauri` helper.
- `vitest-setup.ts` — imports `@testing-library/jest-dom/vitest`.

**Backend (`src-tauri/src/`)**
- `main.rs` — bin: `app_lib::run()`.
- `lib.rs` — `run()`: builder, plugins, setup (db, state, tray, tick spawn), window events, `invoke_handler`.
- `core/mod.rs` — `pub mod clock; pub mod model; pub mod timer; pub mod stats;`.
- `core/clock.rs` — `Clock` trait, `SystemClock`.
- `core/model.rs` — `Routine`, `NewRoutine`, `FocusSession`, `Mode`, `StreakRule`, serde.
- `core/timer.rs` — `TimerEngine`, `TimerState`, `Phase`, `TimerConfig`, `TimerSnapshot`, `TimerEvent`, `CompletedSession`.
- `core/stats.rs` — pure aggregation over `&[FocusSession]`/`&[Routine]`.
- `db/mod.rs` — `open`, `migrate`.
- `db/routines.rs` · `db/sessions.rs` · `db/settings.rs` — repositories.
- `state.rs` — `AppState`, `spawn_tick`.
- `commands.rs` — all `#[tauri::command]` functions.

---

## Phase 0 — Foundation

### Task 1: Scaffold Tauri v2 + convert to plain Svelte SPA

**Files:**
- Create (scaffold): whole project under repo root (`package.json`, `index.html`, `vite.config.ts`, `src/`, `src-tauri/`).
- Modify: `package.json`, `vite.config.ts`, `src-tauri/tauri.conf.json`, `src/App.svelte`, `src/main.ts`.
- Delete: any `svelte.config.js`, `src/routes/+*` (SvelteKit files) if the template emits them.

**Interfaces:**
- Produces: a running app shell (`npm run tauri dev`) with hash routing; `WebviewUrl` route `#/` renders a placeholder.

> ⚠️ The `create-tauri-app` **Svelte** template is SvelteKit-based. We want a plain Svelte + Vite SPA. Steps below scaffold, then strip Kit and add `svelte-spa-router`.

- [ ] **Step 1: Scaffold into the current directory**

Run (from repo root `todo_timer/`):
```bash
npm create tauri-app@latest . -- --template svelte-ts --manager npm --identifier com.minjun.dailyroutinetimer
```
Expected: files created in place. If it refuses because the dir is non-empty, scaffold in a temp dir and move files in, preserving `docs/` and `.git/`.

- [ ] **Step 2: Pin versions in `package.json`**

Set exact versions from Global Constraints. Replace the `dependencies`/`devDependencies` with:
```json
{
  "dependencies": {
    "@tauri-apps/api": "2.11.1",
    "@tauri-apps/plugin-notification": "2.3.3",
    "@tauri-apps/plugin-positioner": "2.3.2",
    "svelte-spa-router": "5.1.1",
    "pretendard": "^1.3.9",
    "@fontsource/jetbrains-mono": "^5.1.0",
    "@fontsource/space-grotesk": "^5.1.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.11",
    "svelte": "5.56.4",
    "vite": "8.1.2",
    "@sveltejs/vite-plugin-svelte": "7.1.2",
    "typescript": "~6.0.3",
    "svelte-check": "^4.6.0",
    "vitest": "4.1.9",
    "@testing-library/svelte": "5.4.2",
    "@testing-library/jest-dom": "^6",
    "jsdom": "29.1.1"
  }
}
```

- [ ] **Step 3: Remove SvelteKit, use plain vite-plugin-svelte**

Delete `svelte.config.js` and any `src/routes/` SvelteKit files. Replace `vite.config.ts`:
```ts
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
});
```
Ensure `index.html` at repo root has `<div id="app"></div>` and `<script type="module" src="/src/main.ts"></script>`.

- [ ] **Step 4: Add hash router + placeholder route**

`src/main.ts`:
```ts
import './app.css';
import App from './App.svelte';
import { mount } from 'svelte';

const app = mount(App, { target: document.getElementById('app')! });
export default app;
```
`src/App.svelte`:
```svelte
<script lang="ts">
  import Router from 'svelte-spa-router';
  import Home from './routes/Home.svelte';
  import Focus from './routes/Focus.svelte';
  import Settings from './routes/Settings.svelte';
  import Report from './routes/Report.svelte';
  import Popover from './routes/Popover.svelte';

  const routes = {
    '/': Home,
    '/focus': Focus,
    '/settings': Settings,
    '/report': Report,
    '/popover': Popover,
  };
</script>

<Router {routes} />
```
Create minimal placeholder `src/routes/{Home,Focus,Settings,Report,Popover}.svelte`, each `<h1>NAME</h1>`. Create empty `src/app.css`.

- [ ] **Step 5: Configure `tauri.conf.json` (two windows, bundle)**

Replace `src-tauri/tauri.conf.json` build/app/bundle to match:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Routine Timer",
  "version": "0.1.0",
  "identifier": "com.minjun.dailyroutinetimer",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      { "label": "main", "url": "index.html#/", "title": "Routine Timer", "width": 900, "height": 640 }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["app", "dmg"],
    "category": "Productivity",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"],
    "macOS": { "minimumSystemVersion": "10.14" }
  }
}
```
(The `popover` window is created at runtime in Task 18, so it is not declared here.)

- [ ] **Step 6: Install + verify build**

Run:
```bash
npm install
npm run tauri dev
```
Expected: app window opens showing "Home" (route `#/`). Close it. Then verify Rust builds standalone:
```bash
cd src-tauri && cargo build && cd ..
```
Expected: `Finished` with no errors.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri v2 + plain Svelte SPA with hash router"
```

---

### Task 2: Vitest harness + first pure util (`formatDuration`)

**Files:**
- Create: `vitest.config.ts` (or fold into `vite.config.ts` test block), `vitest-setup.ts`, `src/lib/time.ts`, `src/lib/time.test.ts`.
- Modify: `package.json` scripts.

**Interfaces:**
- Produces: `formatDuration(totalSeconds: number): string` → `"mm:ss"` (<1h) or `"h:mm:ss"`; `formatClock(date: Date): string` (e.g. `"오전 8:42"` / `"오후 2:14"`).

- [ ] **Step 1: Add Vitest config + setup + test script**

Create `vitest.config.ts`:
```ts
/// <reference types="vitest/config" />
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  test: { environment: 'jsdom', globals: true, setupFiles: ['./vitest-setup.ts'] },
});
```
Create `vitest-setup.ts`:
```ts
import '@testing-library/jest-dom/vitest';
```
Add to `package.json` scripts: `"test": "vitest run"`, `"test:watch": "vitest"`.

- [ ] **Step 2: Write the failing test**

`src/lib/time.test.ts`:
```ts
import { describe, it, expect } from 'vitest';
import { formatDuration } from './time';

describe('formatDuration', () => {
  it('formats mm:ss under an hour', () => {
    expect(formatDuration(0)).toBe('00:00');
    expect(formatDuration(65)).toBe('01:05');
    expect(formatDuration(1500)).toBe('25:00');
  });
  it('formats h:mm:ss at/over an hour', () => {
    expect(formatDuration(3661)).toBe('1:01:01');
  });
  it('clamps negatives to zero', () => {
    expect(formatDuration(-5)).toBe('00:00');
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `npm run test -- time`
Expected: FAIL — `formatDuration` not exported / module not found.

- [ ] **Step 4: Implement**

`src/lib/time.ts`:
```ts
export function formatDuration(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const pad = (n: number) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${pad(m)}:${pad(sec)}`;
}

export function formatClock(date: Date): string {
  const h = date.getHours();
  const m = date.getMinutes();
  const period = h < 12 ? '오전' : '오후';
  const h12 = h % 12 === 0 ? 12 : h % 12;
  return `${period} ${h12}:${String(m).padStart(2, '0')}`;
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npm run test -- time`
Expected: PASS (3 tests).

- [ ] **Step 6: Commit**

```bash
git add vitest.config.ts vitest-setup.ts src/lib/time.ts src/lib/time.test.ts package.json
git commit -m "test: add vitest harness and formatDuration util"
```

---

## Phase 1 — Rust Core (pure, Tauri-free, TDD)

> All Phase 1–2 files live under `src-tauri/src/` and compile into the `app_lib` lib target, so `cargo test` (run from `src-tauri/`) exercises them without any Tauri runtime. Add `pub mod core;`, `pub mod db;`, `pub mod state;`, `pub mod commands;` to `lib.rs` as each is created.

### Task 3: `core::clock` + `core::model`

**Files:**
- Create: `src-tauri/src/core/mod.rs`, `src-tauri/src/core/clock.rs`, `src-tauri/src/core/model.rs`.
- Modify: `src-tauri/src/lib.rs` (add `pub mod core;`).

**Interfaces:**
- Produces:
```rust
// clock.rs
pub trait Clock: Send + Sync { fn now(&self) -> chrono::DateTime<chrono::Utc>; }
pub struct SystemClock;
impl Clock for SystemClock { fn now(&self) -> DateTime<Utc> { Utc::now() } }

// model.rs
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Mode { Pomodoro, Continuous }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StreakRule { Focused, AnyCompleted, AllCompleted }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Routine {
    pub id: i64, pub name: String, pub icon: String, pub color: Option<String>,
    pub target_seconds: i64, pub pomodoro_enabled: bool,
    pub focus_minutes: i64, pub break_minutes: i64,
    pub sort_order: i64, pub archived: bool, pub created_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewRoutine {
    pub name: String, pub icon: String, pub color: Option<String>,
    pub target_seconds: i64, pub pomodoro_enabled: bool,
    pub focus_minutes: i64, pub break_minutes: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FocusSession {
    pub id: i64, pub routine_id: i64,
    pub started_at: DateTime<Utc>, pub ended_at: DateTime<Utc>,
    pub seconds: i64, pub completed: bool,
}
```

- [ ] **Step 1: Write the failing test** — `src-tauri/src/core/model.rs` (append `#[cfg(test)]`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mode_serializes_to_variant_name() {
        assert_eq!(serde_json::to_string(&Mode::Pomodoro).unwrap(), "\"Pomodoro\"");
        assert_eq!(serde_json::to_string(&StreakRule::AllCompleted).unwrap(), "\"AllCompleted\"");
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test model:: 2>&1 | head`. Expected: FAIL (types/`serde_json` missing). Add deps from Global Constraints to `Cargo.toml` and set `[lib] name = "app_lib"` / `crate-type = ["staticlib","cdylib","rlib"]` if not present.

- [ ] **Step 3: Implement** `clock.rs` + `model.rs` with the interface code above, **and create empty stub files** `src-tauri/src/core/timer.rs` and `src-tauri/src/core/stats.rs` (each containing only `// filled in Tasks 4-8`) so the module tree compiles now. `core/mod.rs`:
```rust
pub mod clock;
pub mod model;
pub mod timer;
pub mod stats;
```
Add `use chrono::{DateTime, Utc}; use serde::{Serialize, Deserialize};` where needed.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test model::`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/core src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(core): add Clock trait and data model types"
```

---

### Task 4: `TimerEngine` — continuous countdown (Idle/Running)

**Files:**
- Create: `src-tauri/src/core/timer.rs`.

**Interfaces:**
- Produces (grow across Tasks 4–6):
```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum TimerState { Idle, Running, Paused, Break }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Phase { Focus, Break }
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum TimerEvent { FocusEnded, BreakEnded, TargetReached }

pub struct TimerConfig {
    pub routine_id: i64, pub mode: Mode,
    pub focus_secs: i64, pub break_secs: i64,
    pub target_secs: i64, pub already_done_secs: i64,
}
#[derive(Clone, Debug, Serialize)]
pub struct TimerSnapshot {
    pub state: TimerState, pub mode: Mode, pub phase: Phase,
    pub routine_id: Option<i64>, pub pomodoro_index: u32,
    pub remaining_secs: i64, pub session_seconds: i64,
    pub routine_today_secs: i64, pub target_secs: i64,
    pub state_changed: bool, pub event: Option<TimerEvent>,
    pub remaining_label: String,
}
pub struct CompletedSession {
    pub routine_id: i64, pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>, pub seconds: i64, pub completed: bool,
}

impl TimerEngine {
    pub fn new(clock: Box<dyn Clock>) -> Self;      // Idle
    pub fn start(&mut self, cfg: TimerConfig);       // -> Running (Focus phase)
    pub fn tick(&mut self) -> TimerSnapshot;         // advance 1s, returns snapshot
    pub fn snapshot(&self) -> TimerSnapshot;
    pub fn state(&self) -> TimerState;
    pub fn take_completed(&mut self) -> Option<CompletedSession>; // drains an auto-finalized session
}
```
`remaining_label` = `mm:ss` of `remaining_secs` (reuse formatting logic; see Step 3). **Continuous mode** (per spec §6): `remaining_secs` counts down `target_secs - already_done_secs`; when it hits 0 the engine **auto-finalizes** — transitions to `Idle`, emits `TargetReached` **once**, and stashes a `CompletedSession` retrievable via `take_completed()` (so the tick loop persists it — Task 16). **Pomodoro mode** does NOT auto-stop at target; it keeps cycling (Task 5).

- [ ] **Step 1: Write the failing test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::clock::Clock;
    use chrono::{DateTime, Utc, TimeZone};
    struct FakeClock(DateTime<Utc>);
    impl Clock for FakeClock { fn now(&self) -> DateTime<Utc> { self.0 } }

    fn engine() -> TimerEngine { TimerEngine::new(Box::new(FakeClock(Utc.timestamp_opt(1_700_000_000,0).unwrap()))) }
    fn cont_cfg() -> TimerConfig { TimerConfig { routine_id: 1, mode: Mode::Continuous, focus_secs: 0, break_secs: 0, target_secs: 60, already_done_secs: 0 } }

    #[test]
    fn idle_tick_is_noop() {
        let mut e = engine();
        let snap = e.tick();
        assert_eq!(snap.state, TimerState::Idle);
        assert_eq!(snap.remaining_secs, 0);
    }
    #[test]
    fn continuous_counts_down_and_labels() {
        let mut e = engine();
        e.start(cont_cfg());
        let s = e.snapshot();
        assert_eq!(s.state, TimerState::Running);
        assert_eq!(s.remaining_secs, 60);
        assert_eq!(s.remaining_label, "01:00");
        let s = e.tick();
        assert_eq!(s.remaining_secs, 59);
        assert_eq!(s.session_seconds, 1);
        assert_eq!(s.routine_today_secs, 1);
    }
    #[test]
    fn continuous_target_reached_finalizes_to_idle_and_records() {
        let start_at = Utc.timestamp_opt(1_700_000_000,0).unwrap();
        let mut e = TimerEngine::new(Box::new(FakeClock(start_at)));
        e.start(TimerConfig { target_secs: 2, ..cont_cfg() });
        e.tick();                         // 1 left
        let s = e.tick();                 // 0 -> auto-finalize
        assert_eq!(s.remaining_secs, 0);
        assert_eq!(s.state, TimerState::Idle);
        assert_eq!(s.event, Some(TimerEvent::TargetReached));
        let done = e.take_completed().unwrap();
        assert_eq!(done.seconds, 2);
        assert!(done.completed);
        assert_eq!(done.started_at, start_at);
        let s = e.tick();                 // idle no-op afterwards
        assert_eq!(s.state, TimerState::Idle);
        assert_eq!(s.event, None);
        assert!(e.take_completed().is_none()); // drained
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test timer::`. Expected: FAIL (no `TimerEngine`).

- [ ] **Step 3: Implement** `TimerEngine` for Idle + Running/Continuous. Struct holds `clock`, `state`, `mode`, `phase`, `routine_id: Option<i64>`, `pomodoro_index`, `remaining`, `session_focus_secs`, `already_done_secs`, `target_secs`, `focus_secs`, `break_secs`, `started_at: Option<DateTime<Utc>>`, and `pending_completed: Option<CompletedSession>`. `tick()`: if `Running` and `phase==Focus`, decrement `remaining` (guard `>0`), `session_focus_secs += 1`; in continuous mode when `remaining` reaches 0, **finalize**: set `state=Idle`, `event=TargetReached`, and `pending_completed = Some(CompletedSession { routine_id, started_at, ended_at: clock.now(), seconds: session_focus_secs, completed: true })`. `take_completed()` returns and clears `pending_completed`. Build `remaining_label` with a local `fn label(secs: i64) -> String` identical to `formatDuration` mm:ss/h:mm:ss. `routine_today_secs = already_done_secs + session_focus_secs`.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test timer::`. Expected: PASS (3 tests).

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/core/timer.rs
git commit -m "feat(core): TimerEngine continuous countdown with TargetReached"
```

---

### Task 5: `TimerEngine` — pomodoro focus/break cycle

**Files:** Modify `src-tauri/src/core/timer.rs`.

**Interfaces:** Consumes Task 4 types. Adds pomodoro behavior to `start`/`tick`.

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn pomodoro_cycles_focus_break_and_counts_index() {
    let mut e = TimerEngine::new(Box::new(FakeClock(Utc.timestamp_opt(1_700_000_000,0).unwrap())));
    e.start(TimerConfig { routine_id: 1, mode: Mode::Pomodoro, focus_secs: 2, break_secs: 1, target_secs: 3600, already_done_secs: 0 });
    assert_eq!(e.snapshot().phase, Phase::Focus);
    assert_eq!(e.snapshot().pomodoro_index, 1);
    e.tick();                              // focus 1 left
    let s = e.tick();                      // focus 0 -> break begins
    assert_eq!(s.event, Some(TimerEvent::FocusEnded));
    assert_eq!(s.phase, Phase::Break);
    assert_eq!(s.remaining_secs, 1);
    assert_eq!(s.session_seconds, 2);      // break does not add focus seconds
    let s = e.tick();                      // break 0 -> next focus
    assert_eq!(s.event, Some(TimerEvent::BreakEnded));
    assert_eq!(s.phase, Phase::Focus);
    assert_eq!(s.pomodoro_index, 2);
    assert_eq!(s.remaining_secs, 2);
}
#[test]
fn pomodoro_break_does_not_add_focus_seconds() {
    let mut e = TimerEngine::new(Box::new(FakeClock(Utc.timestamp_opt(1_700_000_000,0).unwrap())));
    e.start(TimerConfig { routine_id: 1, mode: Mode::Pomodoro, focus_secs: 1, break_secs: 2, target_secs: 3600, already_done_secs: 10 });
    e.tick();                              // focus done -> break
    let s = e.tick();                      // in break
    assert_eq!(s.session_seconds, 1);
    assert_eq!(s.routine_today_secs, 11);  // 10 already + 1 focus
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test timer::pomodoro`. Expected: FAIL.

- [ ] **Step 3: Implement** pomodoro transitions in `tick()`: when `phase==Focus` and `remaining` reaches 0 → set `phase=Break`, `remaining=break_secs`, `event=FocusEnded`, `state_changed=true`. When `phase==Break` and `remaining` reaches 0 → `phase=Focus`, `remaining=focus_secs`, `pomodoro_index += 1`, `event=BreakEnded`. Break ticks decrement `remaining` but do **not** increment `session_focus_secs`. `start()` sets `phase=Focus`, `pomodoro_index=1`, `remaining=focus_secs` for pomodoro mode.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test timer::`. Expected: PASS (all).

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/core/timer.rs
git commit -m "feat(core): pomodoro focus/break cycle in TimerEngine"
```

---

### Task 6: `TimerEngine` — pause/resume/stop/skip_break + `CompletedSession`

**Files:** Modify `src-tauri/src/core/timer.rs`.

**Interfaces:** Produces:
```rust
impl TimerEngine {
    pub fn pause(&mut self);
    pub fn resume(&mut self);
    pub fn skip_break(&mut self);
    pub fn stop(&mut self) -> Option<CompletedSession>; // None if was Idle
}
```
`stop()` returns `CompletedSession { routine_id, started_at (from start), ended_at = clock.now(), seconds = session_focus_secs, completed = routine_today_secs >= target_secs }`, then resets engine to Idle.

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn pause_freezes_and_resume_continues() {
    let mut e = engine(); e.start(cont_cfg());
    e.tick();                       // 59
    e.pause();
    assert_eq!(e.state(), TimerState::Paused);
    let s = e.tick();               // paused: no change
    assert_eq!(s.remaining_secs, 59);
    e.resume();
    let s = e.tick();               // 58
    assert_eq!(s.remaining_secs, 58);
}
#[test]
fn stop_returns_completed_session_and_resets() {
    let start_at = Utc.timestamp_opt(1_700_000_000,0).unwrap();
    let mut e = TimerEngine::new(Box::new(FakeClock(start_at)));
    e.start(TimerConfig { target_secs: 3, ..cont_cfg() });
    e.tick(); e.tick(); e.tick();   // 3 focus seconds, target reached
    let done = e.stop().unwrap();
    assert_eq!(done.routine_id, 1);
    assert_eq!(done.seconds, 3);
    assert!(done.completed);
    assert_eq!(done.started_at, start_at);
    assert_eq!(e.state(), TimerState::Idle);
    assert!(e.stop().is_none());     // idle stop -> None
}
#[test]
fn skip_break_jumps_to_next_focus() {
    let mut e = TimerEngine::new(Box::new(FakeClock(Utc.timestamp_opt(1_700_000_000,0).unwrap())));
    e.start(TimerConfig { routine_id: 1, mode: Mode::Pomodoro, focus_secs: 1, break_secs: 30, target_secs: 3600, already_done_secs: 0 });
    e.tick();                        // -> Break (30 left)
    e.skip_break();
    let s = e.snapshot();
    assert_eq!(s.phase, Phase::Focus);
    assert_eq!(s.pomodoro_index, 2);
    assert_eq!(s.remaining_secs, 1);
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test timer::`. Expected: FAIL.

- [ ] **Step 3: Implement** `pause` (Running/Break → Paused, remember prior phase), `resume` (→ Running/Break), `skip_break` (if `phase==Break`: → Focus, `pomodoro_index+=1`, `remaining=focus_secs`), `stop` (build `CompletedSession`, reset to Idle). `tick()` returns unchanged snapshot when `Paused`.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test timer::`. Expected: PASS (all).

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/core/timer.rs
git commit -m "feat(core): pause/resume/stop/skip_break and CompletedSession"
```

---

### Task 7: `core::stats` — daily totals, per-routine progress, completion, remaining

**Files:** Create `src-tauri/src/core/stats.rs`.

**Interfaces:** Produces (deterministic via explicit `tz: FixedOffset`):
```rust
use chrono::{DateTime, Utc, FixedOffset, NaiveDate};
pub fn day_of(dt: DateTime<Utc>, tz: FixedOffset) -> NaiveDate { dt.with_timezone(&tz).date_naive() }
pub fn seconds_per_routine(sessions: &[FocusSession], day: NaiveDate, tz: FixedOffset) -> std::collections::HashMap<i64, i64>;
pub fn today_total(sessions: &[FocusSession], day: NaiveDate, tz: FixedOffset) -> i64;
pub fn completed_count(routines: &[Routine], sessions: &[FocusSession], day: NaiveDate, tz: FixedOffset) -> usize; // routines whose day-sum >= target_seconds (archived excluded)
pub fn remaining_total(routines: &[Routine], sessions: &[FocusSession], day: NaiveDate, tz: FixedOffset) -> i64;   // sum(max(0, target - done)) over active routines
```

- [ ] **Step 1: Write the failing test** (use `FixedOffset::east_opt(0)` = UTC for determinism):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::*;
    use chrono::{Utc, TimeZone, FixedOffset};
    fn utc() -> FixedOffset { FixedOffset::east_opt(0).unwrap() }
    fn sess(routine_id: i64, ts: i64, secs: i64) -> FocusSession {
        let t = Utc.timestamp_opt(ts,0).unwrap();
        FocusSession { id: 0, routine_id, started_at: t, ended_at: t, seconds: secs, completed: false }
    }
    fn routine(id: i64, target: i64) -> Routine {
        Routine { id, name: "r".into(), icon: "x".into(), color: None, target_seconds: target,
                  pomodoro_enabled: true, focus_minutes: 25, break_minutes: 5, sort_order: id, archived: false, created_at: "".into() }
    }
    #[test]
    fn sums_and_completion() {
        let day = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc());
        let base = 1_700_000_000;
        let sessions = vec![sess(1, base, 600), sess(1, base+10, 200), sess(2, base+20, 100)];
        let per = seconds_per_routine(&sessions, day, utc());
        assert_eq!(per.get(&1), Some(&800));
        assert_eq!(today_total(&sessions, day, utc()), 900);
        let routines = vec![routine(1, 800), routine(2, 4000)];
        assert_eq!(completed_count(&routines, &sessions, day, utc()), 1); // routine 1 met 800
        assert_eq!(remaining_total(&routines, &sessions, day, utc()), 3900); // r2: 4000-100
    }
    #[test]
    fn ignores_other_days() {
        let day = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc());
        let yesterday = 1_700_000_000 - 86_400;
        let sessions = vec![sess(1, yesterday, 999)];
        assert_eq!(today_total(&sessions, day, utc()), 0);
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test stats::`. Expected: FAIL.

- [ ] **Step 3: Implement** the functions: group sessions by `day_of(started_at, tz) == day`, sum `seconds`. `completed_count`/`remaining_total` iterate `routines.iter().filter(|r| !r.archived)`.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test stats::`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/core/stats.rs
git commit -m "feat(core): daily totals, completion, remaining stats"
```

---

### Task 8: `core::stats` — streak with 3 rules

**Files:** Modify `src-tauri/src/core/stats.rs`.

**Interfaces:** Produces:
```rust
pub fn streak(routines: &[Routine], sessions: &[FocusSession], rule: StreakRule, today: NaiveDate, tz: FixedOffset) -> u32;
pub fn max_streak(routines: &[Routine], sessions: &[FocusSession], rule: StreakRule, today: NaiveDate, tz: FixedOffset) -> u32; // longest historical run (최고 M일)
```
Counts consecutive days ending at `today` (inclusive) satisfying `rule`:
- `Focused`: that day had ≥1 session with `seconds > 0`.
- `AnyCompleted`: ≥1 active routine's day-sum ≥ its target.
- `AllCompleted`: every active routine's day-sum ≥ its target (and ≥1 active routine exists).
Streak stops at the first day (walking backward) that fails; if `today` itself fails, streak = 0.

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn streak_focused_counts_consecutive_days() {
    let utc = utc();
    let today = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc);
    let base = 1_700_000_000;
    // today, yesterday, day-before => 3 consecutive; skip day-4
    let sessions = vec![
        sess(1, base, 100),
        sess(1, base - 86_400, 100),
        sess(1, base - 2*86_400, 100),
        sess(1, base - 4*86_400, 100),
    ];
    let routines = vec![routine(1, 50)];
    assert_eq!(streak(&routines, &sessions, StreakRule::Focused, today, utc), 3);
}
#[test]
fn streak_zero_when_today_empty() {
    let utc = utc();
    let today = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc);
    let sessions = vec![sess(1, 1_700_000_000 - 86_400, 100)];
    let routines = vec![routine(1, 50)];
    assert_eq!(streak(&routines, &sessions, StreakRule::Focused, today, utc), 0);
}
#[test]
fn streak_all_completed_requires_every_routine() {
    let utc = utc();
    let today = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc);
    let base = 1_700_000_000;
    let sessions = vec![sess(1, base, 100), sess(2, base, 100)]; // only r1 meets target today
    let routines = vec![routine(1, 50), routine(2, 4000)];
    assert_eq!(streak(&routines, &sessions, StreakRule::AllCompleted, today, utc), 0);
    assert_eq!(streak(&routines, &sessions, StreakRule::AnyCompleted, today, utc), 1);
}
#[test]
fn max_streak_finds_longest_past_run() {
    let utc = utc();
    let today = day_of(Utc.timestamp_opt(1_700_000_000,0).unwrap(), utc);
    let base = 1_700_000_000;
    // current run of 1 (today only), older run of 3 (days 5,6,7 back)
    let sessions = vec![
        sess(1, base, 100),
        sess(1, base - 5*86_400, 100),
        sess(1, base - 6*86_400, 100),
        sess(1, base - 7*86_400, 100),
    ];
    let routines = vec![routine(1, 50)];
    assert_eq!(streak(&routines, &sessions, StreakRule::Focused, today, utc), 1);
    assert_eq!(max_streak(&routines, &sessions, StreakRule::Focused, today, utc), 3);
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test stats::`. Expected: FAIL.

- [ ] **Step 3: Implement** `streak`: loop `day` from `today` backward (`day.pred_opt()`), evaluate the rule for that day (reuse `seconds_per_routine`), increment while satisfied, break otherwise. Cap iterations (e.g. 3650) to avoid runaway. Implement `max_streak`: walk backward from `today` to the earliest session day, tracking the current run length and the max seen (reset run to 0 on a non-qualifying day); return the max.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test stats::`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/core/stats.rs
git commit -m "feat(core): streak with focused/any/all rules"
```

---

## Phase 2 — Persistence (SQLite, TDD with in-memory DB)

### Task 9: `db::open` + `db::migrate`

**Files:** Create `src-tauri/src/db/mod.rs`; modify `src-tauri/src/lib.rs` (`pub mod db;`).

**Interfaces:** Produces:
```rust
pub fn open(path: &str) -> rusqlite::Result<rusqlite::Connection>; // path or ":memory:"
pub fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()>;
```
Schema = full spec schema (routine, focus_session, app_settings, schema_version).

- [ ] **Step 1: Write the failing test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrate_creates_tables_idempotently() {
        let conn = open(":memory:").unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap(); // idempotent
        let n: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('routine','focus_session','app_settings')",
            [], |r| r.get(0)).unwrap();
        assert_eq!(n, 3);
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test db::`. Expected: FAIL.

- [ ] **Step 3: Implement:**
```rust
use rusqlite::Connection;
pub fn open(path: &str) -> rusqlite::Result<Connection> { Connection::open(path) }
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS routine (
           id INTEGER PRIMARY KEY, name TEXT NOT NULL, icon TEXT NOT NULL,
           color TEXT, target_seconds INTEGER NOT NULL,
           pomodoro_enabled INTEGER NOT NULL DEFAULT 1,
           focus_minutes INTEGER NOT NULL DEFAULT 25,
           break_minutes INTEGER NOT NULL DEFAULT 5,
           sort_order INTEGER NOT NULL, archived INTEGER NOT NULL DEFAULT 0,
           created_at TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS focus_session (
           id INTEGER PRIMARY KEY, routine_id INTEGER NOT NULL REFERENCES routine(id),
           started_at TEXT NOT NULL, ended_at TEXT NOT NULL,
           seconds INTEGER NOT NULL, completed INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS idx_session_started ON focus_session(started_at);
         CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
    let v: i64 = conn.query_row("SELECT COALESCE(MAX(version),0) FROM schema_version", [], |r| r.get(0)).unwrap_or(0);
    if v < 1 { conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?; }
    Ok(())
}
```
(Note: `:memory:` + WAL is fine for tests; WAL is ignored for in-memory DBs.)

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test db::`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/db/mod.rs src-tauri/src/lib.rs
git commit -m "feat(db): sqlite open + migrate schema"
```

---

### Task 10: `db::routines` — CRUD + reorder

**Files:** Create `src-tauri/src/db/routines.rs`; add `pub mod routines;` to `db/mod.rs`.

**Interfaces:** Produces (all take `&Connection`):
```rust
pub fn create(conn: &Connection, r: &NewRoutine, created_at: &str) -> rusqlite::Result<Routine>;
pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Routine>>;        // active (archived=0), ordered by sort_order
pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Routine>>;
pub fn update(conn: &Connection, r: &Routine) -> rusqlite::Result<()>;
pub fn set_archived(conn: &Connection, id: i64, archived: bool) -> rusqlite::Result<()>;
pub fn reorder(conn: &Connection, ordered_ids: &[i64]) -> rusqlite::Result<()>; // assign sort_order by index
```
`create` assigns `sort_order = COALESCE(MAX(sort_order), 0) + 1` (plain `MAX(...)+1` is NULL on the first insert into an empty table → violates `NOT NULL`); row-mapping helper `fn map_row(row) -> Routine`.

- [ ] **Step 1: Write the failing test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::core::model::NewRoutine;
    fn conn() -> rusqlite::Connection { let c = db::open(":memory:").unwrap(); db::migrate(&c).unwrap(); c }
    fn nr(name: &str) -> NewRoutine { NewRoutine { name: name.into(), icon: "🎯".into(), color: None, target_seconds: 3600, pomodoro_enabled: true, focus_minutes: 25, break_minutes: 5 } }
    #[test]
    fn create_list_update_archive() {
        let c = conn();
        let a = create(&c, &nr("딥워크"), "2026-07-01T00:00:00Z").unwrap();
        let b = create(&c, &nr("외국어"), "2026-07-01T00:00:00Z").unwrap();
        assert_eq!(list(&c).unwrap().len(), 2);
        assert!(b.sort_order > a.sort_order);
        let mut a2 = a.clone(); a2.name = "딥워크2".into(); a2.target_seconds = 7200;
        update(&c, &a2).unwrap();
        assert_eq!(get(&c, a.id).unwrap().unwrap().name, "딥워크2");
        set_archived(&c, a.id, true).unwrap();
        assert_eq!(list(&c).unwrap().len(), 1); // archived excluded
    }
    #[test]
    fn reorder_assigns_sort_order() {
        let c = conn();
        let a = create(&c, &nr("a"), "t").unwrap();
        let b = create(&c, &nr("b"), "t").unwrap();
        reorder(&c, &[b.id, a.id]).unwrap();
        let ids: Vec<i64> = list(&c).unwrap().iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![b.id, a.id]);
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test db::routines`. Expected: FAIL.

- [ ] **Step 3: Implement** the functions with `conn.execute`/`query_row`/`prepare`+`query_map`. Booleans stored as `INTEGER` (`r.pomodoro_enabled as i64`). `reorder` runs updates inside a transaction (`conn.execute` per id, or `let tx = conn.unchecked_transaction()?`).

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test db::routines`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/db/routines.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): routine CRUD + reorder"
```

---

### Task 11: `db::sessions` — insert + range query

**Files:** Create `src-tauri/src/db/sessions.rs`; add `pub mod sessions;`.

**Interfaces:** Produces:
```rust
pub fn insert(conn: &Connection, s: &CompletedSession) -> rusqlite::Result<i64>; // stores RFC3339 UTC strings
pub fn all(conn: &Connection) -> rusqlite::Result<Vec<FocusSession>>;
pub fn since(conn: &Connection, from_rfc3339: &str) -> rusqlite::Result<Vec<FocusSession>>; // started_at >= from
```
Timestamps stored/parsed as RFC3339 (`DateTime::to_rfc3339()` / `DateTime::parse_from_rfc3339(..)?.with_timezone(&Utc)`).

- [ ] **Step 1: Write the failing test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::core::timer::CompletedSession;
    use chrono::{Utc, TimeZone};
    fn conn() -> rusqlite::Connection { let c = db::open(":memory:").unwrap(); db::migrate(&c).unwrap();
        c.execute("INSERT INTO routine (name,icon,target_seconds,sort_order,created_at) VALUES ('r','x',3600,1,'t')", []).unwrap(); c }
    #[test]
    fn insert_and_read_back() {
        let c = conn();
        let t = Utc.timestamp_opt(1_700_000_000,0).unwrap();
        let s = CompletedSession { routine_id: 1, started_at: t, ended_at: t, seconds: 300, completed: true };
        let id = insert(&c, &s).unwrap();
        assert!(id > 0);
        let rows = all(&c).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].seconds, 300);
        assert!(rows[0].completed);
        assert_eq!(rows[0].started_at, t);
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test db::sessions`. Expected: FAIL.

- [ ] **Step 3: Implement** insert/all/since with RFC3339 (de)serialization and `completed as i64`.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test db::sessions`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/db/sessions.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): focus_session insert + queries"
```

---

### Task 12: `db::settings` — get/set with typed defaults

**Files:** Create `src-tauri/src/db/settings.rs`; add `pub mod settings;`.

**Interfaces:** Produces:
```rust
pub fn get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>>;
pub fn set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()>; // upsert
pub fn theme(conn: &Connection) -> rusqlite::Result<String>;       // default "system"
pub fn streak_rule(conn: &Connection) -> rusqlite::Result<String>; // default "focused"
```
Valid `theme`: `"system"|"light"|"dark"`. Valid `streak_rule`: `"focused"|"any_completed"|"all_completed"`.

- [ ] **Step 1: Write the failing test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    fn conn() -> rusqlite::Connection { let c = db::open(":memory:").unwrap(); db::migrate(&c).unwrap(); c }
    #[test]
    fn defaults_then_override() {
        let c = conn();
        assert_eq!(theme(&c).unwrap(), "system");
        assert_eq!(streak_rule(&c).unwrap(), "focused");
        set(&c, "theme", "dark").unwrap();
        set(&c, "theme", "light").unwrap(); // upsert
        assert_eq!(theme(&c).unwrap(), "light");
        assert_eq!(get(&c, "missing").unwrap(), None);
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test db::settings`. Expected: FAIL.

- [ ] **Step 3: Implement** with `INSERT INTO app_settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value`; `theme`/`streak_rule` wrap `get` with the default.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test db::settings`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/db/settings.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): app_settings get/set with defaults"
```

---

## Phase 3 — Tauri Integration

> From here tasks are integration-level. Where pure logic is testable it keeps TDD; window/tray behavior is verified by `cargo build` + the manual checklist (Task 30). Add `use tauri::{Manager, Emitter};` where needed.

### Task 13: `AppState` + routine commands

**Files:** Create `src-tauri/src/state.rs`, `src-tauri/src/commands.rs`; modify `src-tauri/src/lib.rs`.

**Interfaces:** Produces:
```rust
// state.rs
pub struct AppState {
    pub engine: TimerEngine,
    pub db: rusqlite::Connection,
    pub current_routine_name: Option<String>, // for the tray title "딥워크 24:13"
}
// commands.rs — thin wrappers locking State<'_, Mutex<AppState>>
#[tauri::command] pub fn routines_list(state: State<Mutex<AppState>>) -> Result<Vec<Routine>, String>;
#[tauri::command] pub fn routine_create(state: State<Mutex<AppState>>, app: AppHandle, new: NewRoutine) -> Result<Routine, String>;
#[tauri::command] pub fn routine_update(state: State<Mutex<AppState>>, app: AppHandle, routine: Routine) -> Result<(), String>;
#[tauri::command] pub fn routine_delete(state: State<Mutex<AppState>>, app: AppHandle, id: i64) -> Result<(), String>; // set_archived(true)
#[tauri::command] pub fn routine_reorder(state: State<Mutex<AppState>>, app: AppHandle, ordered_ids: Vec<i64>) -> Result<(), String>;
```
Mutation commands emit `routines://changed` (global) after write. `created_at` uses `state.engine`'s clock or `Utc::now().to_rfc3339()`.

- [ ] **Step 1: Write `AppState` construction + a Rust-level test** that exercises the db path (not the command macro):
```rust
// state.rs
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
        let mut st = AppState { engine: TimerEngine::new(Box::new(SystemClock)), db, current_routine_name: None };
        crate::db::routines::create(&st.db, &NewRoutine { name: "딥워크".into(), icon: "🎯".into(), color: None, target_seconds: 3600, pomodoro_enabled: true, focus_minutes: 25, break_minutes: 5 }, "2026-07-01T00:00:00Z").unwrap();
        assert_eq!(crate::db::routines::list(&st.db).unwrap().len(), 1);
        let _ = &mut st.engine; // engine present
    }
}
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test state::`. Expected: FAIL (no `AppState`).

- [ ] **Step 3: Implement** `AppState` (with `current_routine_name: None` at construction), the commands (each: `let s = state.lock().map_err(|e| e.to_string())?;` then call `db::routines::*`; emit via `app.emit("routines://changed", ())`). Register in `lib.rs` `invoke_handler![commands::routines_list, ...]` and `app.manage(Mutex::new(AppState { .. }))` in `setup`.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test state:: && cargo build`. Expected: PASS + build ok.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/state.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(app): AppState + routine commands"
```

---

### Task 14: Timer commands (start/pause/resume/stop/skip_break/switch)

**Files:** Modify `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`.

**Interfaces:** Produces:
```rust
#[tauri::command] pub fn timer_start(state: State<Mutex<AppState>>, app: AppHandle, routine_id: i64) -> Result<TimerSnapshot, String>;
#[tauri::command] pub fn timer_pause(state: State<Mutex<AppState>>) -> Result<TimerSnapshot, String>;
#[tauri::command] pub fn timer_resume(state: State<Mutex<AppState>>) -> Result<TimerSnapshot, String>;
#[tauri::command] pub fn timer_stop(state: State<Mutex<AppState>>, app: AppHandle) -> Result<(), String>;   // persist session
#[tauri::command] pub fn timer_skip_break(state: State<Mutex<AppState>>) -> Result<TimerSnapshot, String>;
#[tauri::command] pub fn timer_switch(state: State<Mutex<AppState>>, app: AppHandle, routine_id: i64) -> Result<TimerSnapshot, String>; // stop+persist then start
#[tauri::command] pub fn timer_get_state(state: State<Mutex<AppState>>) -> Result<TimerSnapshot, String>;
```
`timer_start` reads the routine + today's already-done seconds (via `stats::seconds_per_routine`) to build `TimerConfig`, calls `engine.start`, sets `s.current_routine_name = Some(routine.name.clone())` (used for the tray title in Task 16), and emits `timer://state`. `timer_stop`/`timer_switch`: `if let Some(done) = engine.stop() { db::sessions::insert(&s.db, &done)?; }`, then `timer_stop` sets `s.current_routine_name = None`, `timer_switch` sets it to the new routine's name and starts it; both emit `routines://changed` + `timer://state`.

- [ ] **Step 1: Add a helper + Rust test for the config-building logic** (pure, testable). In `commands.rs` add:
```rust
pub fn build_config(routine: &Routine, already_done: i64) -> TimerConfig {
    TimerConfig {
        routine_id: routine.id,
        mode: if routine.pomodoro_enabled { Mode::Pomodoro } else { Mode::Continuous },
        focus_secs: routine.focus_minutes * 60,
        break_secs: routine.break_minutes * 60,
        target_secs: routine.target_seconds,
        already_done_secs: already_done,
    }
}
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
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test commands::`. Expected: FAIL.

- [ ] **Step 3: Implement** `build_config` + the timer commands using it; register in `invoke_handler`. Compute `already_done` with `let tz = *chrono::Local::now().offset(); let day = stats::day_of(s.engine_clock_now_or_utc(), tz);` — simplest: `let now = Utc::now(); let day = stats::day_of(now, tz);` then `stats::seconds_per_routine(&db::sessions::all(&s.db)?, day, tz).get(&routine_id)`.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test commands:: && cargo build`. Expected: PASS + build ok.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(app): timer commands with session persistence"
```

---

### Task 15: Stats + settings commands

**Files:** Modify `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`.

**Interfaces:** Produces:
```rust
#[derive(Serialize)] pub struct TodayStats {
    pub total_secs: i64, pub completed: usize, pub routine_count: usize,
    pub remaining_secs: i64, pub streak: u32, pub best_streak: u32,
    pub per_routine: std::collections::HashMap<i64, i64>,
}
#[tauri::command] pub fn stats_today(state: State<Mutex<AppState>>) -> Result<TodayStats, String>;
#[tauri::command] pub fn settings_get(state: State<Mutex<AppState>>) -> Result<HashMap<String,String>, String>; // {"theme":..,"streak_rule":..}
#[tauri::command] pub fn settings_set(state: State<Mutex<AppState>>, app: AppHandle, key: String, value: String) -> Result<(), String>; // emits settings://changed
```
`stats_today` reads routines + sessions, computes with `stats::*` using `today = day_of(Utc::now(), Local offset)` and the persisted `streak_rule`.

- [ ] **Step 1: Write the failing test ONLY** (concrete routines/sessions; the `today_stats` helper does not exist yet so this must fail to compile/link):
```rust
#[cfg(test)]
mod stats_tests {
    use super::*;
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
```

- [ ] **Step 2: Run to verify it fails** — `cd src-tauri && cargo test commands::stats_tests`. Expected: FAIL (`today_stats` not found).

- [ ] **Step 3: Implement** the pure helper + the three commands:
```rust
pub fn today_stats(routines: &[Routine], sessions: &[FocusSession], rule: StreakRule, today: NaiveDate, tz: FixedOffset) -> TodayStats {
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
```
The `stats_today` command locks state, reads `db::routines::list` + `db::sessions::all`, parses `db::settings::streak_rule` string → `StreakRule` (`"focused"→Focused`, `"any_completed"→AnyCompleted`, `"all_completed"→AllCompleted`), computes `today = stats::day_of(Utc::now(), *chrono::Local::now().offset())`, and returns `today_stats(...)`. `settings_get`/`settings_set` wrap `db::settings`; `settings_set` emits `settings://changed`. Register all three in `invoke_handler`.

- [ ] **Step 4: Run to verify it passes** — `cd src-tauri && cargo test commands:: && cargo build`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(app): today stats + settings commands"
```

---

### Task 16: Background tick loop (`spawn_tick`)

**Files:** Modify `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`.

**Interfaces:** Produces `pub fn spawn_tick(app: tauri::AppHandle)`; called from `setup`.

Behavior each second, under ONE lock: `snap = engine.tick()`, drain `engine.take_completed()` and (if Some) persist it via `db::sessions::insert` + clear `current_routine_name`, capture the routine name for the tray title, then **drop the guard** before emitting. Emit `timer://tick`; if `snap.state_changed` → `timer://state`; if a session was persisted → `routines://changed`. Tray title = `"딥워크 24:13"` (name + label) when active, else cleared. On `snap.event` fire a notification (plugin wired in Task 19).

- [ ] **Step 1: Implement `spawn_tick`** (no unit test — verified in Task 30):
```rust
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use crate::core::timer::{TimerState, TimerEvent};

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
                let _ = app.notification().builder().title(title).body(body).show();
            }
        }
    });
}
```

- [ ] **Step 2: Wire into `setup`** — after `app.manage(...)`: `spawn_tick(app.handle().clone());`.

- [ ] **Step 3: Verify build** — `cd src-tauri && cargo build`. Expected: `Finished`. (`NotificationExt` requires the plugin registered — do that in Task 19; until then, comment the notification block or land Task 19 first if the build fails on the trait. Recommended: do Task 19's `.plugin(tauri_plugin_notification::init())` before building here.)

- [ ] **Step 4: Commit**
```bash
git add src-tauri/src/state.rs src-tauri/src/lib.rs
git commit -m "feat(app): background 1s tick loop emitting events + tray + notifications"
```

---

## Phase 4 — Menubar Shell

### Task 17: Tray icon + live title + positioner wiring

**Files:** Modify `src-tauri/src/lib.rs`.

**Interfaces:** Builds tray id `"main-tray"` in `setup` (synchronously, main thread). Left-click toggles the popover (Task 18 creates the window; until then, toggle is a no-op guarded by `get_webview_window("popover")`).

- [ ] **Step 1: Implement tray build in `setup`** (icon + live title + right-click menu + left-click popover toggle):
```rust
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri_plugin_positioner::{WindowExt, Position};
// (lib.rs already has `use tauri::Manager;`)

app.handle().plugin(tauri_plugin_positioner::init())?;

// Right-click context menu (spec §4/§8). show_menu_on_left_click(false) keeps left-click for the popover.
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
        "open" => { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }
        "pause" => {
            let state = app.state::<std::sync::Mutex<AppState>>();
            let mut s = state.lock().unwrap();
            match s.engine.state() {
                crate::core::timer::TimerState::Paused => s.engine.resume(),
                crate::core::timer::TimerState::Idle => {}
                _ => s.engine.pause(),
            }
        }
        "quit" => { app.exit(0); }
        _ => {}
    })
    .on_tray_icon_event(|tray, event| {
        tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
            let app = tray.app_handle();
            if let Some(win) = app.get_webview_window("popover") {
                if win.is_visible().unwrap_or(false) { let _ = win.hide(); }
                else { let _ = win.move_window(Position::TrayBottomCenter); let _ = win.show(); let _ = win.set_focus(); }
            }
        }
    })
    .build(app)?;
```

- [ ] **Step 2: Verify build + run** — `cd src-tauri && cargo build && cd ..`; `npm run tauri dev`. Expected: menubar shows `--:--` next to the icon; **right-click** shows the menu (대시보드 열기 / 일시정지·계속 / 종료), 종료 quits the app; left-click does nothing yet (no popover window until Task 18). Dock icon present.

- [ ] **Step 3: Commit**
```bash
git add src-tauri/src/lib.rs
git commit -m "feat(tray): menubar icon with live title + positioner + left-click toggle"
```

---

### Task 18: Popover window (borderless, hidden, auto-hide on blur)

**Files:** Modify `src-tauri/src/lib.rs`.

**Interfaces:** Creates the `popover` WebviewWindow at runtime in `setup` (after tray).

- [ ] **Step 1: Implement popover creation:**
```rust
use tauri::{WebviewUrl, WebviewWindowBuilder};
let _popover = WebviewWindowBuilder::new(app, "popover", WebviewUrl::App("index.html#/popover".into()))
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .resizable(false)
    .inner_size(320.0, 420.0)
    .title("")
    .build()?;
```
> Do **NOT** attach `.on_window_event` on the builder for focus-lost hide: the builder-level closure is `Fn(&WindowEvent)` (no window handle exists at build time). Focus-lost auto-hide is wired at the **app level** in Task 19's `Builder::on_window_event`, whose closure is `Fn(&Window, &WindowEvent)`.

- [ ] **Step 2: Verify** — `npm run tauri dev`. Click the menubar icon → the popover appears under the tray showing the `/popover` placeholder; click elsewhere → it hides; click icon again → toggles. Expected: correct show/hide + placement.

- [ ] **Step 3: Commit**
```bash
git add src-tauri/src/lib.rs
git commit -m "feat(popover): borderless tray-anchored popover window with blur auto-hide"
```

---

### Task 19: Plugins + capabilities + close-to-tray + notification bootstrap

**Files:** Modify `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `src/main.ts`.

**Interfaces:** Registers notification plugin; adds all permissions; main window closes to tray; requests notification permission on startup (JS).

- [ ] **Step 1: Register plugins + app-level window events (close-to-tray + popover auto-hide)** in `run()`:
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_notification::init())
    // positioner plugin is registered in setup (Task 17)
    .on_window_event(|window, event| {
        match window.label() {
            "main" => {
                // close-to-tray: hide instead of quit
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            "popover" => {
                // auto-hide the menubar popover when it loses focus
                if let tauri::WindowEvent::Focused(false) = event { let _ = window.hide(); }
            }
            _ => {}
        }
    })
    // ...setup, invoke_handler, run
```

- [ ] **Step 2: Write full capabilities** — replace `src-tauri/capabilities/default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Daily Routine Timer default capability",
  "windows": ["main", "popover"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:window:allow-set-title",
    "core:window:allow-set-always-on-top",
    "core:window:allow-set-skip-taskbar",
    "core:window:allow-start-dragging",
    "core:event:allow-listen",
    "core:event:allow-emit",
    "core:app:allow-set-app-theme",
    "core:tray:allow-set-title",
    "core:tray:allow-set-icon",
    "core:tray:allow-set-visible",
    "core:menu:default",
    "notification:default",
    "positioner:default"
  ]
}
```
> Note: the tray + context menu are built entirely in Rust (`MenuBuilder`/`TrayIconBuilder`), which does not itself go through the ACL; `core:menu:default` is listed defensively and is harmless.

- [ ] **Step 3: Bootstrap notification permission (JS)** — in `src/main.ts` before mount:
```ts
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification';
async function ensureNotifications() {
  if (!(await isPermissionGranted())) { await requestPermission(); }
}
ensureNotifications();
```

- [ ] **Step 4: Verify** — `npm run tauri dev`. Expected: on first launch macOS asks for notification permission; closing the main window hides it (app stays in menubar/Dock); re-open via Dock. `cargo build` clean (so Task 16's `NotificationExt` now compiles).

- [ ] **Step 5: Commit**
```bash
git add src-tauri/src/lib.rs src-tauri/capabilities/default.json src/main.ts
git commit -m "feat(app): plugins, capabilities, close-to-tray, notification bootstrap"
```

---

## Phase 5 — Frontend Foundation

### Task 20: Shared TS types + typed command wrappers

**Files:** Create `src/lib/types.ts`, `src/lib/commands.ts`.

**Interfaces:** Produces types mirroring Rust payloads and a `commands` object.

- [ ] **Step 1: Implement `src/lib/types.ts`:**
```ts
export type Mode = 'Pomodoro' | 'Continuous';
export type TimerStateName = 'Idle' | 'Running' | 'Paused' | 'Break';
export type Phase = 'Focus' | 'Break';
export type TimerEventName = 'FocusEnded' | 'BreakEnded' | 'TargetReached';
export type StreakRule = 'focused' | 'any_completed' | 'all_completed';
export type ThemePref = 'system' | 'light' | 'dark';

export interface Routine {
  id: number; name: string; icon: string; color: string | null;
  target_seconds: number; pomodoro_enabled: boolean;
  focus_minutes: number; break_minutes: number;
  sort_order: number; archived: boolean; created_at: string;
}
export interface NewRoutine {
  name: string; icon: string; color: string | null;
  target_seconds: number; pomodoro_enabled: boolean;
  focus_minutes: number; break_minutes: number;
}
export interface TimerSnapshot {
  state: TimerStateName; mode: Mode; phase: Phase;
  routine_id: number | null; pomodoro_index: number;
  remaining_secs: number; session_seconds: number;
  routine_today_secs: number; target_secs: number;
  state_changed: boolean; event: TimerEventName | null; remaining_label: string;
}
export interface TodayStats {
  total_secs: number; completed: number; routine_count: number;
  remaining_secs: number; streak: number; best_streak: number; per_routine: Record<number, number>;
}
```

- [ ] **Step 2: Implement `src/lib/commands.ts`:**
```ts
import { invoke } from '@tauri-apps/api/core';
import type { Routine, NewRoutine, TimerSnapshot, TodayStats } from './types';

export const commands = {
  routinesList: () => invoke<Routine[]>('routines_list'),
  routineCreate: (newRoutine: NewRoutine) => invoke<Routine>('routine_create', { new: newRoutine }),
  routineUpdate: (routine: Routine) => invoke<void>('routine_update', { routine }),
  routineDelete: (id: number) => invoke<void>('routine_delete', { id }),
  routineReorder: (orderedIds: number[]) => invoke<void>('routine_reorder', { orderedIds }),
  timerStart: (routineId: number) => invoke<TimerSnapshot>('timer_start', { routineId }),
  timerPause: () => invoke<TimerSnapshot>('timer_pause'),
  timerResume: () => invoke<TimerSnapshot>('timer_resume'),
  timerStop: () => invoke<void>('timer_stop'),
  timerSkipBreak: () => invoke<TimerSnapshot>('timer_skip_break'),
  timerSwitch: (routineId: number) => invoke<TimerSnapshot>('timer_switch', { routineId }),
  timerGetState: () => invoke<TimerSnapshot>('timer_get_state'),
  statsToday: () => invoke<TodayStats>('stats_today'),
  settingsGet: () => invoke<Record<string, string>>('settings_get'),
  settingsSet: (key: string, value: string) => invoke<void>('settings_set', { key, value }),
};
```
> Note: `routine_create`'s Rust param is `new: NewRoutine`, so the JS key is `new`. Keep names aligned.

- [ ] **Step 3: Verify** — `npx tsc --noEmit` (or `npm run check`). Expected: no type errors.

- [ ] **Step 4: Commit**
```bash
git add src/lib/types.ts src/lib/commands.ts
git commit -m "feat(ui): shared types + typed command wrappers"
```

---

### Task 21: Timer store (`timer.svelte.ts`) fed by events

**Files:** Create `src/test/tauri-mock.ts`, `src/lib/timer.svelte.ts`, `src/lib/timer.test.ts`.

**Interfaces:** Produces `timer` singleton + `initTimerListeners(): Promise<UnlistenFn>`.

- [ ] **Step 1: Create the reusable Tauri mock** `src/test/tauri-mock.ts`:
```ts
import { vi } from 'vitest';

// vi.mock() is hoisted above imports; its factory may only reference variables whose
// name starts with `mock`. Hold shared mock state in a `mocks` object via vi.hoisted().
const mocks = vi.hoisted(() => ({
  handlers: new Map<string, (e: unknown) => void>(),
  invoke: vi.fn(() => Promise.resolve()),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (event: string, cb: (e: unknown) => void) => {
    mocks.handlers.set(event, cb);
    return () => mocks.handlers.delete(event);
  }),
}));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    label: 'main',
    theme: () => Promise.resolve('light'),
    onThemeChanged: () => Promise.resolve(() => {}),
    setTheme: () => Promise.resolve(),
  }),
}));

export const invokeMock = mocks.invoke;
export function emitTauri(event: string, payload: unknown) { mocks.handlers.get(event)?.({ event, id: 0, payload }); }
export function resetTauri() {
  mocks.handlers.clear();
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation(() => Promise.resolve());
}
```

- [ ] **Step 2: Write the failing test** `src/lib/timer.test.ts`:
```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { emitTauri, resetTauri } from '../test/tauri-mock';
import { timer, initTimerListeners } from './timer.svelte';

describe('timer store', () => {
  beforeEach(resetTauri);
  it('updates fields and progress from timer://tick', async () => {
    await initTimerListeners();
    emitTauri('timer://tick', {
      state: 'Running', mode: 'Continuous', phase: 'Focus', routine_id: 1, pomodoro_index: 1,
      remaining_secs: 1200, session_seconds: 300, routine_today_secs: 300, target_secs: 1500,
      state_changed: false, event: null, remaining_label: '20:00',
    });
    expect(timer.remainingSecs).toBe(1200);
    expect(timer.routineId).toBe(1);
    expect(timer.progress).toBeCloseTo(0.2);
    expect(timer.label).toBe('20:00');
  });
});
```

- [ ] **Step 3: Run to verify it fails** — `npm run test -- timer.test`. Expected: FAIL.

- [ ] **Step 4: Implement** `src/lib/timer.svelte.ts`:
```ts
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { TimerSnapshot } from './types';

class TimerStore {
  state = $state<TimerSnapshot['state']>('Idle');
  mode = $state<TimerSnapshot['mode']>('Continuous');
  phase = $state<TimerSnapshot['phase']>('Focus');
  routineId = $state<number | null>(null);
  pomodoroIndex = $state(1);
  remainingSecs = $state(0);
  sessionSeconds = $state(0);
  routineTodaySecs = $state(0);
  targetSecs = $state(0);
  label = $state('00:00');

  progress = $derived(this.targetSecs > 0 ? Math.min(1, this.routineTodaySecs / this.targetSecs) : 0);
  isActive = $derived(this.state === 'Running' || this.state === 'Break');

  apply(s: TimerSnapshot) {
    this.state = s.state; this.mode = s.mode; this.phase = s.phase;
    this.routineId = s.routine_id; this.pomodoroIndex = s.pomodoro_index;
    this.remainingSecs = s.remaining_secs; this.sessionSeconds = s.session_seconds;
    this.routineTodaySecs = s.routine_today_secs; this.targetSecs = s.target_secs;
    this.label = s.remaining_label;
  }
}
export const timer = new TimerStore();

export async function initTimerListeners(): Promise<UnlistenFn> {
  const unTick = await listen<TimerSnapshot>('timer://tick', (e) => timer.apply(e.payload));
  const unState = await listen<TimerSnapshot>('timer://state', (e) => timer.apply(e.payload));
  return () => { unTick(); unState(); };
}
```

- [ ] **Step 5: Run to verify it passes** — `npm run test -- timer.test`. Expected: PASS.

- [ ] **Step 6: Commit**
```bash
git add src/test/tauri-mock.ts src/lib/timer.svelte.ts src/lib/timer.test.ts
git commit -m "feat(ui): reactive timer store from Tauri events"
```

---

### Task 22: Routines store (`routines.svelte.ts`)

**Files:** Create `src/lib/routines.svelte.ts`, `src/lib/routines.test.ts`.

**Interfaces:** Produces `routinesStore` with `list: Routine[]`, `stats: TodayStats | null`, `refresh()`, and subscribes to `routines://changed`.

- [ ] **Step 1: Write the failing test** (mock `invoke` to return fixtures):
```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { routinesStore } from './routines.svelte';

describe('routines store', () => {
  beforeEach(resetTauri);
  it('refresh loads routines and stats', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'routines_list') return Promise.resolve([{ id: 1, name: '딥워크', icon: '🎯', color: null, target_seconds: 3600, pomodoro_enabled: true, focus_minutes: 25, break_minutes: 5, sort_order: 1, archived: false, created_at: 't' }]);
      if (cmd === 'stats_today') return Promise.resolve({ total_secs: 900, completed: 0, routine_count: 1, remaining_secs: 2700, streak: 2, per_routine: { 1: 900 } });
      return Promise.resolve();
    });
    await routinesStore.refresh();
    expect(routinesStore.list.length).toBe(1);
    expect(routinesStore.stats?.streak).toBe(2);
    expect(routinesStore.secondsFor(1)).toBe(900);
  });
});
```

- [ ] **Step 2: Run to verify it fails** — `npm run test -- routines.test`. Expected: FAIL.

- [ ] **Step 3: Implement** `src/lib/routines.svelte.ts`:
```ts
import { listen } from '@tauri-apps/api/event';
import { commands } from './commands';
import type { Routine, TodayStats } from './types';

class RoutinesStore {
  list = $state<Routine[]>([]);
  stats = $state<TodayStats | null>(null);
  async refresh() {
    this.list = await commands.routinesList();
    this.stats = await commands.statsToday();
  }
  secondsFor(id: number): number { return this.stats?.per_routine?.[id] ?? 0; }
}
export const routinesStore = new RoutinesStore();

export async function initRoutinesListeners() {
  const un1 = await listen('routines://changed', () => routinesStore.refresh());
  const un2 = await listen('timer://state', () => routinesStore.refresh());
  return () => { un1(); un2(); };
}
```

- [ ] **Step 4: Run to verify it passes** — `npm run test -- routines.test`. Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add src/lib/routines.svelte.ts src/lib/routines.test.ts
git commit -m "feat(ui): routines + today-stats store"
```

---

### Task 23: Theme store (`theme.svelte.ts`)

**Files:** Create `src/lib/theme.svelte.ts`, `src/lib/theme.test.ts`.

**Interfaces:** Produces `themeStore` with `pref: ThemePref`, `effective: 'light'|'dark'`, `setPref(p)`, `init()`. `init()` reads persisted pref + system theme via `getCurrentWindow().theme()`, subscribes to `onThemeChanged`, and sets `document.documentElement.dataset.theme` to the effective theme. When `pref==='system'`, follow system; else force.

- [ ] **Step 1: Confirm the window mock exists** — `src/test/tauri-mock.ts` (Task 21) already mocks `@tauri-apps/api/window` with `getCurrentWindow()` returning `{ label: 'main', theme → 'light', onThemeChanged, setTheme }`. No change needed; just import `resetTauri`/`invokeMock` from it in this test.

- [ ] **Step 2: Write the failing test:**
```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { themeStore } from './theme.svelte';

describe('theme store', () => {
  beforeEach(resetTauri);
  it('system pref follows detected system theme', async () => {
    invokeMock.mockImplementation((cmd: string) => cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused' }) : Promise.resolve());
    await themeStore.init();
    expect(themeStore.pref).toBe('system');
    expect(themeStore.effective).toBe('light'); // system detected as light in mock
    expect(document.documentElement.dataset.theme).toBe('light');
  });
  it('explicit pref overrides system', async () => {
    invokeMock.mockImplementation((cmd: string) => cmd === 'settings_get' ? Promise.resolve({ theme: 'dark', streak_rule: 'focused' }) : Promise.resolve());
    await themeStore.init();
    expect(themeStore.effective).toBe('dark');
  });
});
```

- [ ] **Step 3: Run to verify it fails** — `npm run test -- theme.test`. Expected: FAIL.

- [ ] **Step 4: Implement** `src/lib/theme.svelte.ts` using `getCurrentWindow().theme()`, `onThemeChanged`, `commands.settingsGet/settingsSet`, and applying `document.documentElement.dataset.theme = effective`. `effective = pref === 'system' ? systemTheme : pref`.

- [ ] **Step 5: Run to verify it passes** — `npm run test -- theme.test`. Expected: PASS.

- [ ] **Step 6: Commit**
```bash
git add src/lib/theme.svelte.ts src/lib/theme.test.ts src/test/tauri-mock.ts
git commit -m "feat(ui): theme store with system follow + override"
```

---

### Task 24: Fonts + theme tokens + shared components

**Files:** Modify `src/main.ts`, `src/app.css`; create `src/lib/components/RingTimer.svelte`, `src/lib/components/RoutineCard.svelte`.

**Interfaces:** `RingTimer` props `{ progress: number; label: string; size?: number; accent?: string }`; `RoutineCard` props `{ routine: Routine; todaySecs: number; active?: boolean; onclick?: () => void }` (`active` = this routine's timer is currently running).

> Extract exact colors/spacing from the design files (Global Constraints) before finalizing `app.css`. Below are the structural tokens to fill.

- [ ] **Step 1: Import fonts** in `src/main.ts` (top):
```ts
import 'pretendard/dist/web/variable/pretendardvariable.css';
import '@fontsource/jetbrains-mono/400.css';
import '@fontsource/jetbrains-mono/700.css';
import '@fontsource/space-grotesk/500.css';
```

- [ ] **Step 2: Theme tokens** in `src/app.css` (fill exact hex from design):
```css
:root {
  --font-ui: 'Pretendard Variable', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', ui-monospace, monospace;
  --font-display: 'Space Grotesk', var(--font-ui);
}
:root, [data-theme='light'] {
  --bg: #e7e6e3; --surface: #ffffff; --text: #16181d;
  --muted: rgba(0,0,0,.5); --accent: #2f6bed; --ring-track: rgba(0,0,0,.08);
}
[data-theme='dark'] {
  --bg: #0f1115; --surface: #1a1d24; --text: #f2f3f5;
  --muted: rgba(255,255,255,.55); --accent: #4d82ff; --ring-track: rgba(255,255,255,.1);
}
body { margin: 0; background: var(--bg); color: var(--text); font-family: var(--font-ui); -webkit-font-smoothing: antialiased; }
.timer-numerals { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
```

- [ ] **Step 3: `RingTimer.svelte`** — SVG circle with `stroke-dasharray` progress + centered `.timer-numerals` label:
```svelte
<script lang="ts">
  let { progress = 0, label = '', size = 180, accent = 'var(--accent)' }:
    { progress?: number; label?: string; size?: number; accent?: string } = $props();
  const stroke = 12;
  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
  const offset = $derived(c * (1 - Math.max(0, Math.min(1, progress))));
</script>
<svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
  <circle cx={size/2} cy={size/2} r={r} fill="none" stroke="var(--ring-track)" stroke-width={stroke} />
  <circle cx={size/2} cy={size/2} r={r} fill="none" stroke={accent} stroke-width={stroke}
    stroke-linecap="round" stroke-dasharray={c} stroke-dashoffset={offset}
    transform={`rotate(-90 ${size/2} ${size/2})`} />
  <text x="50%" y="50%" text-anchor="middle" dominant-baseline="central"
    class="timer-numerals" font-size={size*0.22} fill="var(--text)">{label}</text>
</svg>
```

- [ ] **Step 4: `RoutineCard.svelte`** — icon + name + `RingTimer` (progress = `todaySecs/routine.target_seconds`) + status text + remaining. Use `formatDuration`. Wire `onclick`. Status precedence: `active` → `진행 중`; else `todaySecs >= routine.target_seconds` → `완료`; else `todaySecs > 0` → `일부 진행`; else `미시작`. (`active` is passed by the dashboard as `timer.routineId === routine.id && timer.isActive`, so `진행 중` reflects the LIVE timer, not merely accumulated time.)

- [ ] **Step 5: Verify** — `npm run check` + `npm run test`. Expected: type-check clean, existing tests pass. (Visual check happens in Task 30.)

- [ ] **Step 6: Commit**
```bash
git add src/main.ts src/app.css src/lib/components
git commit -m "feat(ui): fonts, theme tokens, RingTimer + RoutineCard components"
```

---

## Phase 6 — Screens

> Each screen wires stores/commands with real logic; **exact layout/styling is extracted from the design files** (Global Constraints). Steps give the functional skeleton + a behavior test where meaningful. Keep Korean copy verbatim.

### Task 25: Dashboard (`Home.svelte`)

**Files:** Replace `src/routes/Home.svelte`.

**Interfaces:** Consumes `routinesStore`, `themeStore`, `initRoutinesListeners`, `RoutineCard`, `formatDuration`, `formatClock`, `commands`, `svelte-spa-router push`.

- [ ] **Step 1: Implement**
  - greeting (by hour: `<12` → `좋은 아침이에요` else `좋은 오후예요`),
  - live clock (`formatClock(new Date())` refreshed each minute via `$effect` + `setInterval`),
  - summary bar `남은 {formatDuration(stats.remaining_secs)} · {routine_count}개 루틴 중 {completed}개 완료`,
  - daily focus total `하루 집중 {formatDuration(stats.total_secs)}` (spec §7 하루 집중 총합),
  - streak `연속 {stats.streak}일 · 최고 {stats.best_streak}일` (spec §7),
  - routine card grid — for each routine render `<RoutineCard {routine} todaySecs={routinesStore.secondsFor(routine.id)} active={timer.routineId === routine.id && timer.isActive} onclick={...} />` (import `timer` from `timer.svelte.ts`),
  - a `새 루틴` button (`push('/settings')`).
  On mount (`$effect`): `routinesStore.refresh()` + `initRoutinesListeners()` (return cleanup). Clicking a `RoutineCard` → `commands.timerStart(routine.id)` then `push('/focus')`.

- [ ] **Step 2: Write a smoke test** `src/routes/Home.test.ts`:
```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import Home from './Home.svelte';

describe('Home', () => {
  beforeEach(() => {
    resetTauri();
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'routines_list' ? Promise.resolve([]) :
      cmd === 'stats_today' ? Promise.resolve({ total_secs: 0, completed: 0, routine_count: 0, remaining_secs: 0, streak: 0, per_routine: {} }) :
      Promise.resolve());
  });
  it('renders a greeting', async () => {
    const { findByText } = render(Home);
    expect(await findByText(/좋은 (아침이에요|오후예요)/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Run test** — `npm run test -- Home.test`. Expected: PASS. Fix wiring until green.

- [ ] **Step 4: Commit**
```bash
git add src/routes/Home.svelte src/routes/Home.test.ts
git commit -m "feat(ui): dashboard screen"
```

---

### Task 26: Focus screen (`Focus.svelte`)

**Files:** Replace `src/routes/Focus.svelte`.

**Interfaces:** Consumes `timer` store, `initTimerListeners`, `commands`, `RingTimer`, `push`.

- [ ] **Step 1: Implement**
  - big `RingTimer` (progress `timer.progress`, label `timer.label`),
  - pomodoro header `집중 중 · 포모도로 {timer.pomodoroIndex}` when `mode==='Pomodoro'`; Break state shows `휴식`,
  - the current routine's config subtitle `{routine.focus_minutes}분 집중 · {routine.break_minutes}분 휴식` when Pomodoro (spec §7) — look up the routine via `routinesStore.list.find(r => r.id === timer.routineId)`,
  - controls: `일시정지`/`계속하기` (toggle `commands.timerPause()`/`timerResume()` by `timer.state`), `세션 종료` (`commands.timerStop()` → `push('/')`), `뒤로` (`push('/')`), and in Break a `건너뛰기` (`commands.timerSkipBreak()`),
  - **다음 제안** (spec §7): when in Break OR when `timer.state === 'Idle'` (session just finished), compute the next incomplete active routine — the first `routinesStore.list` item (excluding the current) whose `routinesStore.secondsFor(r.id) < r.target_seconds` — and show a `다음: {name}` button that calls `commands.timerSwitch(r.id)`.
  On mount (`$effect`): `routinesStore.refresh()`, `commands.timerGetState()` to seed the store, and `initTimerListeners()` (cleanup on destroy).

- [ ] **Step 2: Write a behavior test** — pause button calls `timer_pause`:
```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import { invokeMock, emitTauri, resetTauri } from '../test/tauri-mock';
import Focus from './Focus.svelte';

describe('Focus', () => {
  beforeEach(resetTauri);
  it('pause button invokes timer_pause', async () => {
    invokeMock.mockImplementation((cmd: string) => cmd === 'timer_get_state'
      ? Promise.resolve({ state: 'Running', mode: 'Continuous', phase: 'Focus', routine_id: 1, pomodoro_index: 1, remaining_secs: 60, session_seconds: 0, routine_today_secs: 0, target_secs: 60, state_changed: false, event: null, remaining_label: '01:00' })
      : Promise.resolve());
    const { findByText } = render(Focus);
    const btn = await findByText('일시정지');
    await fireEvent.click(btn);
    expect(invokeMock).toHaveBeenCalledWith('timer_pause');
  });
});
```
(The wrapper `timerPause: () => invoke('timer_pause')` calls `invoke` with a single argument, so the recorded call is `['timer_pause']` — assert with the single arg, not `('timer_pause', undefined)`.)

- [ ] **Step 3: Run test** — `npm run test -- Focus.test`. Expected: PASS.

- [ ] **Step 4: Commit**
```bash
git add src/routes/Focus.svelte src/routes/Focus.test.ts
git commit -m "feat(ui): focus screen with pomodoro controls"
```

---

### Task 27: Settings + routine editor (`Settings.svelte`)

**Files:** Replace `src/routes/Settings.svelte`.

**Interfaces:** Consumes `routinesStore`, `themeStore`, `commands`.

- [ ] **Step 1: Implement** two sections:
  1. **Routine editor**: list existing routines with edit/delete **and reorder** (`↑`/`↓` buttons that swap adjacent items and call `commands.routineReorder(orderedIds)` with the new id order, then `routinesStore.refresh()` — spec §7 정렬); a form to create/update (`이름`, `아이콘` emoji text input, `요구 시간` hours+minutes → `target_seconds`, `포모도로` toggle → `pomodoro_enabled`, `집중 분`/`휴식 분`). Save → `commands.routineCreate`/`routineUpdate` → `routinesStore.refresh()`. Delete → `commands.routineDelete`.
  2. **Preferences**: theme radio (`시스템`/`라이트`/`다크`) → `themeStore.setPref` (persists via `settings_set`); streak-rule select (`집중한 날`/`루틴 1개+ 완성`/`모든 루틴 완성`) → `commands.settingsSet('streak_rule', value)`.

- [ ] **Step 2: Write a behavior test** — creating a routine invokes `routine_create`:
```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import Settings from './Settings.svelte';

describe('Settings', () => {
  beforeEach(() => { resetTauri(); invokeMock.mockImplementation((cmd: string) =>
    cmd === 'routines_list' ? Promise.resolve([]) :
    cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused' }) :
    cmd === 'stats_today' ? Promise.resolve({ total_secs:0, completed:0, routine_count:0, remaining_secs:0, streak:0, per_routine:{} }) :
    Promise.resolve()); });
  it('submitting the new-routine form invokes routine_create', async () => {
    const { findByLabelText, findByText } = render(Settings);
    await fireEvent.input(await findByLabelText('이름'), { target: { value: '독서' } });
    await fireEvent.click(await findByText('저장'));
    expect(invokeMock.mock.calls.some(c => c[0] === 'routine_create')).toBe(true);
  });
});
```
(Adjust label/button text to your markup, kept consistent with the assertions.)

- [ ] **Step 3: Run test** — `npm run test -- Settings.test`. Expected: PASS.

- [ ] **Step 4: Commit**
```bash
git add src/routes/Settings.svelte src/routes/Settings.test.ts
git commit -m "feat(ui): settings + routine editor"
```

---

### Task 28: Popover screen (`Popover.svelte`) + Report stub

**Files:** Replace `src/routes/Popover.svelte`, `src/routes/Report.svelte`.

**Interfaces:** Popover consumes `timer`, `routinesStore`, `commands`; compact layout for the 320×420 window.

- [ ] **Step 1: Implement `Popover.svelte`** — compact status: today summary (`남은 {formatDuration(stats.remaining_secs)} · {completed}개 완료`), current routine + `RingTimer` when active, and quick actions: if active → `일시정지`/`계속`; else a short list of routines to start (`commands.timerStart(id)`). On mount (`$effect`): `themeStore.init()` (the popover is a separate webview — it MUST theme itself so it matches the main window, spec §9), `routinesStore.refresh()`, `commands.timerGetState()`, and `initTimerListeners()` (return cleanup).

- [ ] **Step 2: Implement `Report.svelte` stub** — heading `집중 기록` + copy `리포트는 곧 추가됩니다` (deferred slice). Keeps the route valid so nav never breaks.

- [ ] **Step 3: Verify** — `npm run check` + `npm run test`. Expected: clean + green.

- [ ] **Step 4: Commit**
```bash
git add src/routes/Popover.svelte src/routes/Report.svelte
git commit -m "feat(ui): popover screen + report stub"
```

---

## Phase 7 — Wire-up & Verification

### Task 29: Boot sequence + theme/routines init in `App.svelte`

**Files:** Modify `src/App.svelte`.

**Interfaces:** Ensures `themeStore.init()` and `routinesStore.refresh()`/listeners run once for the main window (NOT the popover, which does its own light init).

- [ ] **Step 1: Implement** an `$effect` in `App.svelte` that, when the current window label is `main` (`getCurrentWindow().label`), calls `themeStore.init()` and `initRoutinesListeners()` and returns their cleanup. The popover route handles its own init in Task 28.

- [ ] **Step 2: Verify** — `npm run check` + `npm run test`. Expected: clean + green.

- [ ] **Step 3: Commit**
```bash
git add src/App.svelte
git commit -m "feat(ui): app boot sequence (theme + routines init)"
```

---

### Task 30: End-to-end manual verification + fix pass

**Files:** none (verification); fixes committed as found.

- [ ] **Step 1: Run the full test + build gates**
```bash
npm run test            # all Vitest green
cd src-tauri && cargo test && cargo build && cd ..   # all cargo tests green, build ok
npm run tauri dev
```

- [ ] **Step 2: Manual checklist** (verify each; file+fix any failure, then re-commit):
  - Dashboard shows greeting, clock, empty-state; add 2 routines in Settings; cards appear with rings.
  - Click a routine → Focus screen; countdown ticks; menubar title updates each second.
  - Pomodoro routine: focus→break transition fires a notification; `포모도로 N` increments; `건너뛰기` skips break.
  - Continuous routine (pomodoro off): counts down to target; `목표 달성` notification once.
  - `일시정지`/`계속하기` freeze/resume; menubar reflects state.
  - `세션 종료` → returns to dashboard; the routine's ring reflects accumulated time; completion count updates.
  - Menubar click → popover appears under the icon with correct status; click away → hides.
  - Close main window → app stays in menubar/Dock; reopen via Dock.
  - Settings: switch theme `시스템`/`라이트`/`다크` → both windows update; toggle at OS level while in `시스템` → follows. Change streak rule → dashboard streak recomputes.
  - Tray right-click menu (built in Task 17): `대시보드 열기` shows the main window; `일시정지 / 계속` toggles the timer; `종료` fully exits the app.

- [ ] **Step 3: Commit any fixes**
```bash
git add -A
git commit -m "fix: address manual verification findings"
```

- [ ] **Step 4: Finish** — invoke `superpowers:finishing-a-development-branch` to decide merge/PR/cleanup.

---

## Notes for the implementer

- **Order matters for build health:** Task 16's notification code needs the plugin registered (Task 19). If executing strictly in order, either land Task 19's `.plugin(tauri_plugin_notification::init())` line early, or keep the notification block commented until Task 19. Flagged inline in Task 16 Step 3.
- **`timer_get_state`** must return the current snapshot even when Idle (used to seed Focus/Popover on mount).
- **Local day** for stats uses `*chrono::Local::now().offset()` as the `FixedOffset` at call time; pure stats functions stay deterministic because the offset is passed in (tests use UTC).
- **Do not** hold the `AppState` mutex guard across `.await` in the tick loop — lock, snapshot, drop, then emit (Task 16 shows the pattern).
- **Popover least privilege:** the single capability grants both windows; acceptable for MVP. Split into a popover-scoped capability later if desired.
