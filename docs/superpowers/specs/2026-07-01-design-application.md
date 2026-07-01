# Daily Routine Timer — Design Application Spec

Applies the real design from the claude_design canvas (`Daily Routine Timer.dc.html`) to the built app. Full extraction: workflow output at `.superpowers/sdd/` (design-extract). This doc is the implementer reference.

## Decisions (confirmed with user)
- **Tabs: 오늘 (dashboard `/`) · 리포트 (report `/report`) · 설정 (settings `/settings`)** — 3 tabs (design ships 2; user wants a Settings tab too).
- **새 루틴 = full-screen MODAL** opened from the dashboard "오늘의 루틴" header "+새 루틴" pill (NOT a tab, NOT inside Settings). This is how new-routine is "separated from settings".
- **Settings tab** = theme (light/dark/system) + streak-rule + routine management (edit/delete/reorder of existing routines). Routine CREATION lives only in the modal.
- **포모도로 toggle defaults ON** in the new-routine modal.
- Keep the existing Rust backend + stores/commands unchanged; this is a **frontend visual + structure** change only. Report stays a styled stub (full report/heatmap is a later slice).
- Main window keeps native decorations; our own top bar holds the centered tab pill (traffic-light dots are the OS's). Borderless custom titlebar = future refinement.

## Design tokens — write to `src/app.css` (drives `[data-theme='light'|'dark']`)

`themeStore.apply()` sets `document.documentElement.dataset.theme`. Define all tokens under `:root, [data-theme='light']` and override under `[data-theme='dark']`.

| token | light | dark |
|---|---|---|
| `--bg` | `#ffffff` | `#0a0c11` |
| `--chrome` | `#fcfcfb` | `#0c0e13` |
| `--card` | `#ffffff` | `#13161d` |
| `--today-card` | `#f5f7fa` | `#11141b` |
| `--ink` | `#191b20` | `#eef1f8` |
| `--muted` | `#73726b` | `#8a90a0` |
| `--faint` | `#a9a79f` | `#6b7283` |
| `--faint2` | `#bdbbb3` | `#6b7283` |
| `--accent` | `#2f6bed` | `#5b8def` |
| `--accent-bg` | `#eef3fe` | `#15233e` |
| `--border` | `#eceef2` | `#1f2530` |
| `--hair` | `#f1f0eb` | `#1c212b` |
| `--track` | `#eef1f6` | `#171b24` |
| `--ring-track` | `#e7ebf1` | `#171b24` |
| `--active-bg` | `#f3f7ff` | `#13203a` |
| `--active-border` | `#dde9ff` | `#2b4a86` |
| `--active-track` | `#e1eaff` | `#1c2c4a` |
| `--chev` | `#c2c0b8` | `#525a68` |
| `--row-hover` | `#f6f8fb` | `#161b24` |
| `--dot-empty` | `#dfe3ea` | `#272e3a` |
| `--radial` | `rgba(47,107,237,.10)` | `rgba(91,141,239,.22)` |
| `--ring-glow` | `drop-shadow(0 0 4px rgba(47,107,237,.28))` | `drop-shadow(0 0 8px rgba(91,141,239,.9))` |
| `--btn-glow` | `0 14px 28px -8px rgba(47,107,237,.5)` | `0 0 32px rgba(91,141,239,.6)` |

- Fonts (already imported): `--font-ui:'Pretendard Variable',system-ui,sans-serif`; `--font-mono:'JetBrains Mono',ui-monospace,monospace`; `--font-display:'Space Grotesk',var(--font-ui)`.
- Radius: cards `14px`, buttons/inputs/icon-tiles `12px`, pills `20px`, chips `9px`, progress bars `3px`.
- Card shadow (light, subtle): rely on `1px solid var(--border)` hairline, not heavy shadow. Window/frame shadow not needed (native window).
- `body{ background:var(--bg); color:var(--ink); font-family:var(--font-ui); }`. `.mono{ font-family:var(--font-mono); font-variant-numeric:tabular-nums }`. All numerals `tabular-nums`.
- Traffic-light dots (if drawn): `#ff5f57 / #febc2e / #28c840` (decorative only; OS draws real ones).

## App shell / nav (`App.svelte`)
- A top bar (~44px, `background:var(--chrome)`, `border-bottom:1px var(--hair)`) with a **centered segmented tab pill**: container `background:var(--track)`, `border-radius:11px`, small padding; 3 segments 오늘 / 리포트 / 설정. Active segment = raised pill `background:var(--card)`, `color:var(--ink)`, subtle shadow/border; inactive `color:var(--faint)`. Tab label `600 11.5px`.
- Sync to route via `svelte-spa-router`: `import { location, push } from 'svelte-spa-router'`; active tab = current `$location` (`/`→오늘, `/report`→리포트, `/settings`→설정); click → `push(path)`.
- Router content area below the bar (scrollable, `background:var(--bg)`).
- The bar shows only for `main` window (not the popover — popover has no tabs). Gate on `getCurrentWindow().label === 'main'`, else render just `<Router/>`.

## Screens (per design)

### 오늘 / Dashboard (`Home.svelte`) — content column max ~544px, centered, ~28px side padding
1. Uppercase date eyebrow (e.g. `6월 30일 화요일`), color `--faint`, letter-spacing, 11px.
2. Greeting `좋은 아침이에요.` — `600 25px`, `--ink`.
3. Subtext `오늘 벌써 {mins}분 집중했어요.` with the minutes in `--accent`, `--muted`.
4. **오늘의 목표 card** (`--card`, radius 14, 1px `--border`, padding ~20): left = 80px SVG ring showing overall % (accent arc on `--ring-track`, stroke ~9, rounded cap, center % badge `700 17px`); right = label `오늘의 목표`(`--faint` 12px) + goal `{done} / {target}`(done bold `--ink`, "/ target" `--faint2`, ~20px) + a 5px horizontal progress bar (accent fill on `--track`, radius 3) + caption `남은 {remaining} · {count}개 루틴 중 {completed}개 완료`(`--faint` 12px).
5. **오늘의 루틴 section**: header row = title `오늘의 루틴`(`600 15px`) + right-aligned accent pill button `+ 새 루틴`(bg `--accent-bg`, color `--accent`, radius 20, `600 12px`) → opens the new-routine modal.
6. **Routine rows** (vertical list, NOT grid): each row (`--card`, radius 12, 1px `--border`, hover `--row-hover`, padding ~12–14) = left icon tile (34px rounded-square, `--accent-bg` bg, emoji) + name (`600 14px`) + optional status badge (`진행 중`=accent-bg/accent pill; `완료`=check) + a thin progress bar (accent on `--track`) + time `{done} / {required}`(`--faint2` mono-ish) + chevron `--chev`. **Active/running row**: `background:var(--active-bg)`, `border:1px var(--active-border)`, bar track `--active-track`. Click a row → `commands.timerStart(id)` + `push('/focus')`.

### 새 루틴 MODAL (`components/NewRoutineModal.svelte`) — full-screen overlay
- Overlay covers the content (or whole window): `background:var(--bg)`, own header (centered title `새 루틴`, `600 13px`; a 취소 / close on the left or a top bar). Fields:
  - **이름** — text input (`--today-card` fill, radius 12, `--border`).
  - **아이콘** — a 6-emoji picker grid; selected = `--accent-bg` bg + `--accent` ring.
  - **요구 시간** — a big value display (`600 27px`, e.g. `1시간 30분`) + duration chips `30분 / 45분 / 1시간 / 1시간 30분 / 2시간` (selected chip = accent-bg/accent).
  - **포모도로** — a toggle switch, **DEFAULT ON**, subtitle `25분 집중 · 5분 휴식`. (When on → `pomodoro_enabled:true`, `focus_minutes:25`, `break_minutes:5`.)
  - Footer: `취소` (outline) + `루틴 추가` (solid accent) → `commands.routineCreate({name, icon, color:null, target_seconds, pomodoro_enabled, focus_minutes, break_minutes})` → `routinesStore.refresh()` → close modal.
- Modal open state lives in the dashboard (`let showNew = $state(false)`); the pill sets it true; the modal's 취소/추가 set it false.

### 포커스 / Focus (`Focus.svelte`)
- Back pill top-left (`대시보드`/`뒤로`, radius 8, `--card`/`--border`).
- Radial glow behind ring (`--radial`, radial-gradient).
- Big SVG ring: r≈110, stroke-width 12, track `--ring-track`, arc `--accent` with `filter:var(--ring-glow)`, rounded linecap, `transition:stroke-dashoffset 1s linear`. Center: JetBrains Mono mm:ss `600 50px`, letter-spacing -.01em; caption `남은 시간 · {label}`.
- Eyebrow above ring (uppercase `지금 집중`/`집중 중`, `--faint`) + routine name (`600 22px`).
- **Pomodoro**: a row of session dots (filled = accent + `filter` glow, empty = `--dot-empty`) + label `{index} / N 세션 · 포모도로`; and the config subtitle `{focus}분 집중 · {break}분 휴식`.
- Today mini card (`--today-card`, radius 12): `오늘 {routine} {done} / {target}` + a mini progress bar.
- **3-button control row**: two 48px circular OUTLINE buttons (`--border`, `--card`) flanking a 66px SOLID-accent circular play/pause button (`--accent`, white icon, `box-shadow:var(--btn-glow)`). Map: left = 세션 종료(stop) or 뒤로; center = 일시정지/계속(pause/resume, or start); right = 건너뛰기(skip, when Break) or next-suggestion. Keep all existing commands (timerPause/Resume/Stop/SkipBreak/Switch) + 다음 제안 logic; just restyle to this control layout.

### 리포트 / Report (`Report.svelte`) — styled STUB
- Keep as a placeholder (full heatmap/KPIs are a later slice) but style it to the token system: a `--card` with `집중 기록` heading (`600 20px`) + `리포트는 곧 추가됩니다` (`--muted`). Optional: a faint disabled preview. Do NOT build the real charts now.

### 설정 / Settings (`Settings.svelte`)
- Design has no settings screen — design it to the token system, sectioned cards:
  - **테마** section: a segmented control 시스템 / 라이트 / 다크 bound to `themeStore.pref` → `themeStore.setPref(...)`.
  - **연속(streak) 규칙** section: a select/segmented 집중한 날 / 루틴 1개+ 완성 / 모든 루틴 완성 → `commands.settingsSet('streak_rule', ...)`.
  - **루틴 관리** section: the list of routines with edit / delete / reorder ↑↓ (keep the existing logic — routineUpdate/routineDelete/routineReorder). Routine CREATION is NOT here (it's the dashboard modal).

### 팝오버 / Popover (`Popover.svelte`) — 320×420
- Match the design's popover card: `--card` bg, header row = 34px rounded-square icon chip (`--accent-bg`) + current routine name + status; body = today summary `남은 {remaining} · {completed}개 완료` + a small ring/bar when active; quick actions (일시정지/계속 or a short start list). Keep existing store wiring.

## Verification
- `bun run check` 0 errors, `bun run test` green (existing tests must still pass; update any test whose DOM assumptions changed — e.g. the dashboard "새 루틴" is now a modal-opening pill, greeting unchanged; Settings no longer has the create form so its create-invokes test must move to the modal's test). `bun run build` clean. `cd src-tauri && cargo test && cargo build` unaffected (no backend change) but run to confirm.
- Manual GUI (`bun run tauri dev`) after: tab pill switches 오늘/리포트/설정; +새 루틴 opens modal with 포모도로 ON by default; light/dark/system theme visibly re-skins everything.
