<script lang="ts">
  import { timer, initTimerListeners } from '$lib/timer.svelte';
  import { routinesStore, initRoutinesListeners } from '$lib/routines.svelte';
  import { themeStore } from '$lib/theme.svelte';
  import { commands } from '$lib/commands';
  import { formatDuration } from '$lib/time';
  import RingTimer from '$lib/components/RingTimer.svelte';

  const stats = $derived(routinesStore.stats);
  const routine = $derived(routinesStore.list.find((r) => r.id === timer.routineId));

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
  <!-- Today summary -->
  <div class="summary">
    {#if stats}
      <p class="summary-text">남은 {formatDuration(stats.remaining_secs)} · {stats.completed}개 완료</p>
    {:else}
      <p class="summary-text summary-loading">로딩 중…</p>
    {/if}
  </div>

  {#if timer.isActive}
    <!-- Active state: current routine + ring timer + pause/resume -->
    <div class="active-view">
      {#if routine}
        <p class="routine-name">{routine.icon} {routine.name}</p>
      {/if}
      <div class="ring-wrapper">
        <RingTimer progress={timer.progress} label={timer.label} size={140} />
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
    gap: 12px;
    padding: 16px;
    width: 100%;
    min-height: 100%;
    box-sizing: border-box;
    background: var(--bg, #fff);
    color: var(--text, #111);
  }

  .summary {
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border, #e5e7eb);
  }

  .summary-text {
    margin: 0;
    font-size: 0.85rem;
    color: var(--muted, #6b7280);
    font-weight: 500;
  }

  .summary-loading {
    opacity: 0.5;
  }

  /* Active view */
  .active-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  .routine-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text, #111);
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
    padding: 8px 20px;
    border: none;
    border-radius: 8px;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .btn-primary {
    background: var(--accent, #4f6ef7);
    color: #fff;
  }

  .btn-secondary {
    background: var(--surface, #f3f4f6);
    color: var(--text, #111);
  }

  /* Inactive: routine list */
  .routine-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .routine-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: var(--surface, #f9fafb);
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 8px;
    font-size: 0.9rem;
    color: var(--text, #111);
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.1s;
  }

  .routine-btn:hover {
    background: var(--surface-hover, #f0f4ff);
  }

  .routine-icon {
    font-size: 1.1rem;
  }

  .routine-label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    margin: 0;
    font-size: 0.85rem;
    color: var(--muted, #9ca3af);
    text-align: center;
    padding: 16px 0;
  }
</style>
