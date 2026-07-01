<script lang="ts">
  import { timer, initTimerListeners } from '$lib/timer.svelte';
  import { routinesStore, initRoutinesListeners } from '$lib/routines.svelte';
  import { themeStore } from '$lib/theme.svelte';
  import { commands } from '$lib/commands';
  import { formatDurationKo } from '$lib/time';
  import RoutineRow from '$lib/components/RoutineRow.svelte';
  import FocusView from '$lib/components/FocusView.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  // Which view the popover shows: focus (default when a session is active) or dashboard.
  let view = $state('dashboard');

  const stats = $derived(routinesStore.stats);
  const totalTarget = $derived(routinesStore.list.reduce((acc, r) => acc + r.target_seconds, 0));
  const overallPercent = $derived(
    stats && totalTarget > 0 ? Math.min(1, stats.total_secs / totalTarget) : 0
  );

  // Compact SVG ring constants (smaller sibling of Home's 오늘의 목표 ring)
  const RING_SIZE = 64;
  const RING_STROKE = 7;
  const RING_R = (RING_SIZE - RING_STROKE) / 2;
  const RING_C = 2 * Math.PI * RING_R;

  // Alive-flag async cleanup pattern (spec §9)
  $effect(() => {
    let alive = true;
    let timerCleanup: (() => void) | undefined;
    let routinesCleanup: (() => void) | undefined;
    let focusCleanup: (() => void) | undefined;
    themeStore.init();
    routinesStore.refresh();
    commands.timerGetState().then((s) => {
      if (alive) {
        timer.apply(s);
        view = timer.isActive ? 'focus' : 'dashboard';
      }
    });
    initTimerListeners().then((fn) => { if (alive) timerCleanup = fn; else fn(); });
    initRoutinesListeners().then((fn) => { if (alive) routinesCleanup = fn; else fn(); });
    // Each time the popover is shown (window gains focus), snap back to the running timer.
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused && timer.isActive) view = 'focus';
      })
      .then((fn) => { if (alive) focusCleanup = fn; else fn(); });
    return () => { alive = false; timerCleanup?.(); routinesCleanup?.(); focusCleanup?.(); };
  });

  async function start(id: number) {
    // Don't restart an already-running/paused session (would reset the pomodoro block).
    if (!(timer.routineId === id && timer.state !== 'Idle')) {
      await commands.timerStart(id);
    }
    view = 'focus';
  }
</script>

<div class="popover-shell">
  <div class="popover-body">
  {#if view === 'focus'}
    <FocusView onBack={() => (view = 'dashboard')} size={180} />
  {:else}
    <div class="popover">
  <!-- 오늘의 목표 summary -->
  {#if stats}
    <div class="goal-card">
      <svg width={RING_SIZE} height={RING_SIZE} class="ring-svg" aria-hidden="true">
        <circle
          cx={RING_SIZE / 2} cy={RING_SIZE / 2} r={RING_R}
          fill="none"
          stroke="var(--ring-track)"
          stroke-width={RING_STROKE}
        />
        <circle
          cx={RING_SIZE / 2} cy={RING_SIZE / 2} r={RING_R}
          fill="none"
          stroke="var(--accent)"
          stroke-width={RING_STROKE}
          stroke-linecap="round"
          stroke-dasharray={RING_C}
          stroke-dashoffset={RING_C * (1 - overallPercent)}
          transform={`rotate(-90 ${RING_SIZE / 2} ${RING_SIZE / 2})`}
        />
        <text
          x={RING_SIZE / 2} y={RING_SIZE / 2}
          text-anchor="middle"
          dominant-baseline="central"
          font-size="11"
          font-weight="700"
          fill="var(--ink)"
          font-family="var(--font-ui)"
        >{Math.round(overallPercent * 100)}%</text>
      </svg>

      <div class="goal-info">
        <p class="goal-label">오늘의 목표</p>
        <p class="goal-numbers">
          <span class="goal-done">{formatDurationKo(stats.total_secs)}</span><!--
          --><span class="goal-target"> / {formatDurationKo(totalTarget)}</span>
        </p>
        <p class="goal-caption">남은 {formatDurationKo(stats.remaining_secs)} · {stats.completed}개 완료</p>
      </div>
    </div>
  {:else}
    <div class="goal-card goal-loading">
      <p class="summary-loading">로딩 중…</p>
    </div>
  {/if}

  <!-- 오늘의 루틴 -->
  <div class="routines-section">
    <h2 class="section-title">오늘의 루틴</h2>
    <div class="routine-list">
      {#if routinesStore.list.length === 0}
        <p class="empty-state">루틴이 없습니다</p>
      {:else}
        {#each routinesStore.list as routine (routine.id)}
          <RoutineRow
            {routine}
            todaySecs={routinesStore.secondsFor(routine.id)}
            active={timer.routineId === routine.id && timer.isActive}
            onclick={() => start(routine.id)}
          />
        {/each}
      {/if}
    </div>
  </div>
    </div>
  {/if}
  </div>
</div>

<style>
  /* Transparent window so only the rounded card shows */
  :global(html),
  :global(body) {
    background: transparent;
  }

  /* Rounded + shadowed outer shell (margin leaves room for the drop shadow) */
  .popover-shell {
    margin: 10px;
    border-radius: 16px;
    box-shadow: 0 20px 50px -15px rgba(10, 12, 20, 0.45);
    background: var(--bg);
    overflow: hidden;
    height: calc(100vh - 20px);
    box-sizing: border-box;
  }

  /* Single scroll container for both views (handles a long routine list / tall focus view) */
  .popover-body {
    height: 100%;
    overflow-y: auto;
  }

  .popover {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px;
    width: 100%;
    min-height: 100%;
    box-sizing: border-box;
    background: var(--bg);
    color: var(--ink);
    font-family: var(--font-ui);
  }

  /* 오늘의 목표 card */
  .goal-card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    padding: 14px;
    display: flex;
    align-items: center;
    gap: 14px;
    flex-shrink: 0;
  }
  .goal-loading {
    justify-content: center;
    padding: 20px 14px;
  }
  .summary-loading {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
    opacity: 0.7;
  }
  .ring-svg {
    flex-shrink: 0;
  }
  .goal-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .goal-label {
    margin: 0;
    font-size: 11px;
    color: var(--faint);
  }
  .goal-numbers {
    margin: 0;
    font-size: 15px;
    line-height: 1.2;
  }
  .goal-done {
    font-weight: 700;
    color: var(--ink);
  }
  .goal-target {
    font-size: 12px;
    color: var(--faint2);
  }
  .goal-caption {
    margin: 0;
    font-size: 11px;
    color: var(--faint);
  }

  /* 오늘의 루틴 section */
  .routines-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }
  .section-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
  }
  .routine-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .empty-state {
    margin: 0;
    padding: 16px;
    text-align: center;
    font-size: 13px;
    color: var(--faint);
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
  }
</style>
