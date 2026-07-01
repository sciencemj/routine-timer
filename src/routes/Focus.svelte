<script lang="ts">
  import { push } from 'svelte-spa-router';
  import { timer, initTimerListeners } from '$lib/timer.svelte';
  import { routinesStore } from '$lib/routines.svelte';
  import { commands } from '$lib/commands';
  import RingTimer from '$lib/components/RingTimer.svelte';

  // Look up current routine from the store
  const routine = $derived(routinesStore.list.find((r) => r.id === timer.routineId));

  // Next suggestion: first incomplete active routine (excluding current)
  const nextRoutine = $derived.by(() => {
    if (timer.phase !== 'Break' && timer.state !== 'Idle') return null;
    return routinesStore.list.find(
      (r) => r.id !== timer.routineId && routinesStore.secondsFor(r.id) < r.target_seconds
    ) ?? null;
  });

  // Alive-flag async cleanup pattern
  $effect(() => {
    let alive = true;
    let cleanup: (() => void) | undefined;
    routinesStore.refresh();
    commands.timerGetState().then((s) => { if (alive) timer.apply(s); });
    initTimerListeners().then((fn) => { if (alive) cleanup = fn; else fn(); });
    return () => { alive = false; cleanup?.(); };
  });

  async function handlePauseResume() {
    if (timer.state === 'Paused') {
      await commands.timerResume();
    } else {
      await commands.timerPause();
    }
  }

  async function handleStop() {
    await commands.timerStop();
    push('/');
  }

  async function handleSkipBreak() {
    await commands.timerSkipBreak();
  }

  async function handleSwitch(routineId: number) {
    await commands.timerSwitch(routineId);
  }
</script>

<div class="focus">
  {#if timer.mode === 'Pomodoro'}
    <header class="pomodoro-header">
      <p class="pomodoro-title">
        {#if timer.phase === 'Break'}
          휴식
        {:else}
          집중 중 · 포모도로 {timer.pomodoroIndex}
        {/if}
      </p>
      {#if routine}
        <p class="pomodoro-config">{routine.focus_minutes}분 집중 · {routine.break_minutes}분 휴식</p>
      {/if}
    </header>
  {/if}

  <div class="ring-wrapper">
    <RingTimer progress={timer.progress} label={timer.label} size={220} />
  </div>

  <div class="controls">
    <button class="btn-primary" onclick={handlePauseResume}>
      {timer.state === 'Paused' ? '계속하기' : '일시정지'}
    </button>

    {#if timer.phase === 'Break'}
      <button class="btn-secondary" onclick={handleSkipBreak}>건너뛰기</button>
    {/if}

    <button class="btn-danger" onclick={handleStop}>세션 종료</button>

    <button class="btn-ghost" onclick={() => push('/')}>뒤로</button>
  </div>

  {#if nextRoutine}
    <div class="next-suggestion">
      <button class="btn-next" onclick={() => handleSwitch(nextRoutine.id)}>
        다음: {nextRoutine.name}
      </button>
    </div>
  {/if}
</div>

<style>
  .focus {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 24px;
    padding: 32px 24px;
  }

  .pomodoro-header {
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .pomodoro-title {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--text);
  }

  .pomodoro-config {
    margin: 0;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .ring-wrapper {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .controls {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    width: 100%;
    max-width: 240px;
  }

  .controls button {
    width: 100%;
    padding: 10px 20px;
    border: none;
    border-radius: 8px;
    font-size: 0.95rem;
    cursor: pointer;
  }

  .btn-primary {
    background: var(--accent, #4f6ef7);
    color: #fff;
  }

  .btn-secondary {
    background: var(--surface, #e8eaf6);
    color: var(--text);
  }

  .btn-danger {
    background: #e53935;
    color: #fff;
  }

  .btn-ghost {
    background: transparent;
    color: var(--muted);
    border: 1px solid var(--border, #ccc) !important;
  }

  .next-suggestion {
    width: 100%;
    max-width: 240px;
  }

  .btn-next {
    width: 100%;
    padding: 10px 20px;
    background: var(--surface, #f0f4ff);
    color: var(--accent, #4f6ef7);
    border: 1px solid var(--accent, #4f6ef7);
    border-radius: 8px;
    font-size: 0.9rem;
    cursor: pointer;
  }
</style>
