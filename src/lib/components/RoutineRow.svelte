<script lang="ts">
  import type { Routine } from '../types';
  import { formatDurationKo } from '../time';

  let {
    routine,
    todaySecs,
    active = false,
    onclick,
    editing = false,
    onEdit,
    onDelete,
    index = 0,
    onHandleDown,
    dropTarget = false,
  }: {
    routine: Routine;
    todaySecs: number;
    active?: boolean;
    onclick?: () => void;
    editing?: boolean;
    onEdit?: () => void;
    onDelete?: () => void;
    index?: number;
    onHandleDown?: (e: PointerEvent) => void;
    dropTarget?: boolean;
  } = $props();

  const progress = $derived(routine.target_seconds > 0 ? Math.min(1, todaySecs / routine.target_seconds) : 0);
  const completed = $derived(routine.target_seconds > 0 && todaySecs >= routine.target_seconds);

  function handlePrimary() {
    (editing ? onEdit : onclick)?.();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handlePrimary();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="routine-row"
  class:active
  class:editing
  class:drop-target={dropTarget}
  role="button"
  tabindex="0"
  data-routine-index={index}
  onclick={handlePrimary}
  onkeydown={handleKeydown}
>
  <div class="icon-tile">{routine.icon}</div>
  <div class="row-main">
    <div class="row-top">
      <span class="name">{routine.name}</span>
      {#if active}
        <span class="badge badge-active">진행 중</span>
      {:else if completed}
        <span class="badge badge-done">✓</span>
      {/if}
    </div>
    <div class="progress-bar-track">
      <div class="progress-bar-fill" style="width: {progress * 100}%"></div>
    </div>
    <span class="time-text mono">{formatDurationKo(todaySecs)} / {formatDurationKo(routine.target_seconds)}</span>
  </div>
  {#if editing}
    <div class="edit-controls">
      <button
        type="button"
        class="drag-handle"
        aria-label="드래그로 순서 변경"
        onpointerdown={(e) => { e.preventDefault(); e.stopPropagation(); onHandleDown?.(e); }}
      >⠿</button>
      <button
        type="button"
        class="icon-btn danger"
        onclick={(e) => { e.stopPropagation(); onDelete?.(); }}
        aria-label="삭제"
      >🗑</button>
    </div>
  {:else}
    <span class="chev" aria-hidden="true">›</span>
  {/if}
</div>

<style>
  .routine-row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px 14px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--r-btn);
    cursor: pointer;
    text-align: left;
    font-family: var(--font-ui);
    transition: background 150ms;
  }
  .routine-row:hover {
    background: var(--row-hover);
  }
  .routine-row.active {
    background: var(--active-bg);
    border-color: var(--active-border);
  }
  /* Straight insertion indicator (sits in the 8px gap above the row) */
  .routine-row.drop-target::before {
    content: '';
    position: absolute;
    top: -5px;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent);
  }
  .icon-tile {
    width: 34px;
    height: 34px;
    min-width: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-bg);
    border-radius: var(--r-btn);
    font-size: 18px;
    line-height: 1;
  }
  .row-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  .row-top {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .name {
    font-size: 14px;
    font-weight: 600;
    color: var(--ink);
  }
  .badge {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--r-pill);
  }
  .badge-active {
    background: var(--accent-bg);
    color: var(--accent);
  }
  .badge-done {
    color: var(--accent);
  }
  .progress-bar-track {
    width: 100%;
    height: 4px;
    background: var(--track);
    border-radius: var(--r-bar);
    overflow: hidden;
  }
  .routine-row.active .progress-bar-track {
    background: var(--active-track);
  }
  .progress-bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--r-bar);
    transition: width 300ms;
  }
  .time-text {
    font-size: 12px;
    color: var(--faint2);
  }
  .chev {
    font-size: 20px;
    color: var(--chev);
    line-height: 1;
  }
  .edit-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .drag-handle {
    background: none;
    border: none;
    padding: 0 2px;
    color: var(--chev);
    font-size: 15px;
    line-height: 1;
    cursor: grab;
    touch-action: none;
    user-select: none;
  }
  .drag-handle:active {
    cursor: grabbing;
  }
  .icon-btn {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: var(--r-chip);
    background: var(--today-card);
    color: var(--ink);
    font-size: 13px;
    line-height: 1;
    cursor: pointer;
    padding: 0;
  }
  .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .icon-btn.danger {
    color: #e5484d;
  }
</style>
