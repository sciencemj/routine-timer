<script lang="ts">
  import type { Routine } from '../types';
  import { formatDuration } from '../time';
  import RingTimer from './RingTimer.svelte';

  let { routine, todaySecs, active = false, onclick }: {
    routine: Routine;
    todaySecs: number;
    active?: boolean;
    onclick?: () => void;
  } = $props();

  const progress = $derived(routine.target_seconds > 0 ? todaySecs / routine.target_seconds : 0);
  const remaining = $derived(Math.max(0, routine.target_seconds - todaySecs));
  const label = $derived(formatDuration(active ? remaining : todaySecs));
  const status = $derived(
    active ? '진행 중' :
    todaySecs >= routine.target_seconds ? '완료' :
    todaySecs > 0 ? '일부 진행' : '미시작'
  );
</script>
<button class="routine-card" {onclick}>
  <span class="icon">{routine.icon}</span>
  <span class="name">{routine.name}</span>
  <RingTimer {progress} {label} />
  <span class="status">{status}</span>
</button>

<style>
  .routine-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 16px;
    background: var(--surface);
    border: none;
    border-radius: 12px;
    cursor: pointer;
    color: var(--text);
    font-family: var(--font-ui);
  }
  .icon { font-size: 1.5rem; }
  .name { font-weight: 600; font-size: 0.95rem; }
  .status { font-size: 0.8rem; color: var(--muted); }
</style>
