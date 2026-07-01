# PASS A — Design Tokens + App Shell/Tab-Nav + RingTimer Restyle

## 1. Token CSS (`src/app.css`)

Box-sizing reset + radius scale (`--r-card:14px`, `--r-btn:12px`, `--r-pill:20px`, `--r-chip:9px`, `--r-bar:3px`) in `:root`. Font vars (`--font-ui`, `--font-mono`, `--font-display`) in `:root`. Full token set under `:root, [data-theme='light']` and dark overrides under `[data-theme='dark']` — 21 color/effect tokens each matching the spec table exactly. `body` rule uses `var(--bg)`, `var(--ink)`, `var(--font-ui)`. `.mono` utility replaces old `.timer-numerals` with `font-family:var(--font-mono); font-variant-numeric:tabular-nums`.

Notable: old tokens (`--surface`, `--text`, `--ring-track` old values) removed; all replaced by spec values.

## 2. App shell / tab-nav (`src/App.svelte`)

- `isMain = getCurrentWindow().label === 'main'` computed once at module eval; `$effect` gated on `isMain` (equivalent to original label check).
- For main window: `.app-shell` flex-column fills `100vh`; `.top-bar` (44px, `var(--chrome)` bg, `1px solid var(--hair)` bottom border, centered) holds the `.tab-pill` (3 segments: 오늘/리포트/설정).
- Tab pill container: `var(--track)` bg, `border-radius:11px`, 3px padding, 2px gap.
- Segment buttons: `600 11.5px`, inactive `var(--faint)`, active raised with `var(--card)` bg + `var(--ink)` color + subtle shadow. `transition` on color/bg/shadow 0.15s.
- Active detection: `router.location === tab.path` using the `router` export from `svelte-spa-router` (v5-style reactive state object — the old Svelte 3/4 `location` store is not exported in this version).
- `.content` area: `flex:1; overflow-y:auto; background:var(--bg)` below the bar.
- Popover window renders bare `<Router {routes} />` with no bar.

## 3. RingTimer restyle (`src/lib/components/RingTimer.svelte`)

- Track circle: `stroke="var(--ring-track)"` (unchanged prop).
- Progress arc: moved to class `.arc` with `transition:stroke-dashoffset 1s linear` and `filter:var(--ring-glow)` in a `<style>` block.
- `stroke-linecap="round"` already present; kept.
- Label text: class changed from `timer-numerals` to `mono` (matches new utility), fill changed from `var(--text)` to `var(--ink)`.
- Props API unchanged: `progress`, `label`, `size`, `accent` — all existing consumers unaffected.

## 4. Verification results

```
bun run check   → 0 errors  0 warnings  230 files checked
bun run test    → 11 test files  30 tests  all PASSED  (unchanged count)
bun run build   → clean, 143 modules, dist/ written
```

## 5. Concerns / notes

- `svelte-spa-router` in this project uses a Svelte 5–style `router` reactive state object rather than a legacy Svelte store; `router.location` is reactive via runes. This is correct for this version but differs from the spec's suggested `import { location }` (which would be a type error). No functional difference.
- The active tab highlight for `/focus` and `/popover` routes won't match any of the three tab segments — those routes are entered programmatically and the pill will show no active segment, which is the correct UX (focus screen overlays the whole window anyway).
- Light mode `--bg: #ffffff` and `--card: #ffffff` are identical; card borders (`1px solid var(--border)`) provide the visual separation.
