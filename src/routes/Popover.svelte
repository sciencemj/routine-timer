<script lang="ts">
  import { timer, initTimerListeners } from '$lib/timer.svelte';
  import { routinesStore, initRoutinesListeners } from '$lib/routines.svelte';
  import { themeStore } from '$lib/theme.svelte';
  import { commands } from '$lib/commands';
  import { formatDurationKo } from '$lib/time';
  import RingTimer from '$lib/components/RingTimer.svelte';

  const stats = $derived(routinesStore.stats);
  const routine = $derived(routinesStore.list.find((r) => r.id === timer.routineId));

  const headerIcon = $derived(routine?.icon ?? '⏱️');
  const headerName = $derived(routine?.name ?? '루틴 타이머');
  const headerStatus = $derived(
    timer.isActive ? (timer.phase === 'Break' ? '휴식 중' : '집중 중') : '루틴을 선택해 시작하세요'
  );

  // Alive-flag async cleanup pattern (spec §9)
  $effect(() => {
    let alive = true;
    let timerCleanup: (() => void) | undefined;
    let routinesCleanup: (() => void) | undefined;
    themeStore.init();
    routinesStore.refresh();
    commands.timerGetState().then((s) => { if (alive) timer.apply(s); });
    initTimerListeners().then((fn) => { if (alive) timerCleanup = fn; else fn(); });
    initRoutinesListeners().then((fn) => { if (alive) routinesCleanup = fn; else fn(); });
    return () => { alive = false; timerCleanup?.(); routinesCleanup?.(); };
  });
</script>

<div class="popover">
  <!-- Header: current routine chip + name + status -->
  <div class="header">
    <div class="icon-chip">{headerIcon}</div>
    <div class="header-text">
      <p class="header-name">{headerName}</p>
      <p class="header-status">{headerStatus}</p>
    </div>
  </div>

  <!-- Today summary -->
  <div class="summary">
    {#if stats}
      <p class="summary-text">남은 {formatDurationKo(stats.remaining_secs)} · {stats.completed}개 완료</p>
    {:else}
      <p class="summary-text summary-loading">로딩 중…</p>
    {/if}
  </div>

  {#if timer.isActive}
    <!-- Active state: ring timer + pause/resume -->
    <div class="active-view">
      <div class="ring-wrapper">
        <RingTimer progress={timer.progress} label={timer.label} size={120} />
      </div>
      <div class="controls">
        {#if timer.state === 'Paused'}
          <button class="btn-primary" onclick={() => commands.timerResume()}>계속</button>
        {:else}
          <button class="btn-secondary" onclick={() => commands.timerPause()}>일시정지</button>
        {/if}
      </div>
    </div>
  {:else}
    <!-- Inactive state: quick-start list -->
    <div class="routine-list">
      {#if routinesStore.list.length === 0}
        <p class="empty">루틴이 없습니다</p>
      {:else}
        {#each routinesStore.list as r (r.id)}
          <button class="routine-btn" onclick={() => commands.timerStart(r.id)}>
            <span class="routine-icon">{r.icon}</span>
            <span class="routine-label">{r.name}</span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
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

  /* Header */
  .header {
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    padding: 10px 12px;
  }
  .icon-chip {
    width: 34px;
    height: 34px;
    min-width: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-bg);
    border-radius: 10px;
    font-size: 17px;
    line-height: 1;
  }
  .header-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .header-name {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .header-status {
    margin: 0;
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Today summary */
  .summary-text {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
    font-weight: 500;
  }
  .summary-loading {
    opacity: 0.6;
  }

  /* Active view */
  .active-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding-top: 4px;
  }

  .ring-wrapper {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .controls {
    display: flex;
    gap: 8px;
  }

  .btn-primary,
  .btn-secondary {
    padding: 8px 22px;
    border: none;
    border-radius: var(--r-btn);
    font-size: 13px;
    font-weight: 600;
    font-family: var(--font-ui);
    cursor: pointer;
    transition: opacity 150ms;
  }
  .btn-primary:hover,
  .btn-secondary:hover {
    opacity: 0.85;
  }

  .btn-primary {
    background: var(--accent);
    color: #fff;
  }

  .btn-secondary {
    background: var(--accent-bg);
    color: var(--accent);
  }

  /* Inactive: quick-start routine list */
  .routine-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .routine-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 11px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-btn);
    font-size: 13px;
    font-weight: 500;
    color: var(--ink);
    font-family: var(--font-ui);
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 150ms;
  }

  .routine-btn:hover {
    background: var(--row-hover);
  }

  .routine-icon {
    font-size: 16px;
    line-height: 1;
  }

  .routine-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    margin: 0;
    font-size: 13px;
    color: var(--faint);
    text-align: center;
    padding: 16px 0;
  }
</style>
