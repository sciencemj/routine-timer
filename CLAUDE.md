# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Routine Timer — a macOS-first desktop app for managing daily routines via a circular focus timer + a menubar countdown. **Tauri v2** (Rust) + **Svelte 5** (runes). UI is Korean.

## Commands

Use **bun**, not npm (`tauri.conf.json` drives dev/build through `bun run`).

```bash
bun install                       # deps
bun run tauri dev                 # run the full app (Rust + Vite, hot reload)
bun run dev                       # Vite frontend only (no Tauri shell)

bun run check                     # svelte-check (types) — must be 0 errors / 0 warnings
bun run test                      # vitest (frontend), single run
bun run test:watch                # vitest watch
bunx vitest run src/lib/time.test.ts            # a single test file
bunx vitest run -t "formatDurationKo"           # tests matching a name

bun run build                     # vite build → dist/
bun run tauri build               # bundle .app + .dmg → src-tauri/target/release/bundle/

cd src-tauri && cargo test        # Rust core + db + commands tests
cd src-tauri && cargo test day_of # a single Rust test by name
cd src-tauri && cargo build       # must be 0 warnings

bun run tauri icon <path/to/1024.png>   # regenerate the whole icon set from a source PNG
```

Before committing, expect all gates green: `bun run check` (0/0), `bun run test`, `bun run build`, `cargo test`, `cargo build`.

## Architecture

Two runtimes bridged by Tauri **commands** (frontend → Rust request/response) and **events** (Rust → frontend push). The whole app is one Vite bundle served into **two windows**.

### Rust backend (`src-tauri/src/`)

- **`core/`** is pure and framework-free (no Tauri deps) so it is unit-testable: `timer.rs` (the `TimerEngine` state machine — Idle/Running/Paused + Focus/Break phases, Pomodoro vs Continuous mode), `stats.rs` (day-bucketed aggregation), `model.rs`, and `clock.rs` (a `Clock` trait so tests inject deterministic time).
- **`db/`** = `rusqlite` access. Table names are **singular**: `routine`, `focus_session`, `app_settings` (key/value). Timestamps are RFC3339.
- **`state.rs`** = `AppState { engine, db, current_routine_name }` held behind a `Mutex`, plus `spawn_tick` — a background tokio loop that advances the engine every second, persists completed sessions, updates the tray title, and emits state.
- **`commands.rs`** = the Tauri command layer. Non-obvious invariants to preserve:
  - **Drop the `Mutex` guard before `app.emit(...)`** — compute inside a `{ ... }` block that returns the snapshot, then emit after the guard drops. The tick loop likewise must never hold the guard across an `await`.
  - **`day_context(db)`** returns `(today, tz)` where `tz` is the real local `FixedOffset` shifted earlier by the configured `day_start_hour` (default 8). ALL day bucketing (`stats::day_of` + everything downstream) goes through this, so "the day" starts at 8 AM, not midnight. Any new stats/command that buckets by day MUST use `day_context`, never raw `Local::now()`.
  - **Pomodoro suspend/resume**: leaving a running Pomodoro routine saves its block (`remaining_secs`/phase/index) into the `app_settings` key `pomo_states` (JSON map keyed by routine id); starting a routine restores it via `TimerConfig.resume` and clears the entry. Clicking the already-running routine is a no-op start (guarded), so it doesn't reset the block.
- **`lib.rs`** = setup: tray icon with a live `set_title` countdown (`icon_as_template(true)` → menubar auto-inverts), a borderless **transparent** popover `WebviewWindow` (needs `macOSPrivateApi` + the `macos-private-api` cargo feature), close-to-tray (`CloseRequested` → hide), and the `Overlay` title bar. Events emitted: `timer://state` (a `TimerSnapshot`) and `routines://changed`.

### Svelte frontend (`src/`)

- **Svelte 5 runes** (`$state`/`$derived`/`$effect`) — only valid in `.svelte` / `.svelte.ts` files.
- **Stores** (`src/lib/*.svelte.ts`): `timer`, `routinesStore`, `themeStore` are factory objects wrapping runes. They subscribe to the Rust events via `init*Listeners()`, called from a component's mount `$effect`.
- **Alive-flag async cleanup pattern**: mount effects that await must guard writes with a local `let alive = true` and `return () => { alive = false; ... }` — see any route's mount `$effect`. Follow it for new async-on-mount code.
- **One bundle, two windows**: `App.svelte` gates on `getCurrentWindow().label === 'main'` → the main window renders the top-bar tab shell (오늘 `/` · 리포트 `/report` · 설정 `/settings`) around a `svelte-spa-router` `<Router>`; the `popover` window renders a bare `<Router>` (route `/popover`). `FocusView.svelte` is shared by the `/focus` route and the popover.
- **`commands.ts`** = typed `invoke` wrappers. Note `routine_create` passes its arg under the key `new` (matches the Rust param name).
- **Durations** are formatted in Korean via `time.ts` `formatDurationKo` ("1시간 30분"); clock mm:ss via `formatDuration`.
- **`$lib` alias** must stay defined in all three of `vite.config.ts`, `vitest.config.ts`, and `tsconfig.json` (paths, no baseUrl).

### Tests

- Frontend uses `src/test/tauri-mock.ts` to mock `@tauri-apps/api` (`invoke` / `event.listen` / `getCurrentWindow`). Because `vi.mock` is hoisted, shared mock state lives in a `vi.hoisted(() => ({...}))` object whose name must start with `mock`. Call `resetTauri()` (and `resetTimer()` where present) in `beforeEach`. To assert a command fired, check `invokeMock`; to simulate a Rust push, call `emitTauri(event, payload)`.
- Rust core tests use `FixedOffset::east_opt(0)` (UTC) + the injectable `Clock`, so day-boundary/timer logic is deterministic.
