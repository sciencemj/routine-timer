<script lang="ts">
  import { timer } from '$lib/timer.svelte';
  import { routinesStore } from '$lib/routines.svelte';
  import { commands } from '$lib/commands';
  import { formatDuration } from '$lib/time';
  import RingTimer from '$lib/components/RingTimer.svelte';

  interface Props {
    onBack: () => void;
    size?: number;
  }
  let { onBack, size = 246 }: Props = $props();

  // Look up current routine from the store
  const routine = $derived(routinesStore.list?.find((r) => r.id === timer.routineId));

  // Eyebrow above the ring
  const eyebrow = $derived(timer.phase === 'Break' ? '휴식' : '집중 중');

  // Pomodoro session dots: total capped at 8, filled up to pomodoroIndex
  const dotCount = $derived.by(() => {
    if (!routine) return 0;
    const per = Math.max(1, routine.focus_minutes) * 60;
    return Math.min(8, Math.max(1, Math.ceil(routine.target_seconds / per)));
  });
  const filledCount = $derived(Math.min(Math.max(0, timer.pomodoroIndex), dotCount));

  // Next suggestion: first incomplete active routine (excluding current)
  const nextRoutine = $derived(
    routinesStore.list?.find(
      (r) => r.id !== timer.routineId && routinesStore.secondsFor(r.id) < r.target_seconds
    ) ?? null
  );

  // Center control accessible name (icon-only button)
  const centerLabel = $derived(timer.state === 'Paused' ? '계속하기' : '일시정지');

  async function handlePauseResume() {
    if (timer.state === 'Paused') {
      await commands.timerResume();
    } else {
      await commands.timerPause();
    }
  }

  async function handleStop() {
    await commands.timerStop();
    onBack();
  }

  async function handleSkipBreak() {
    await commands.timerSkipBreak();
  }

  async function handleSwitch(routineId: number) {
    await commands.timerSwitch(routineId);
  }
</script>

<div class="focus">
  <button class="back-pill" onclick={onBack}>뒤로</button>

  <div class="stage">
    <!-- Eyebrow + routine name -->
    <div class="head">
      <p class="eyebrow">{eyebrow}</p>
      <h1 class="routine-name">{routine?.name ?? ''}</h1>
    </div>

    <!-- Ring with radial glow -->
    <div class="ring-stage">
      <div class="radial-glow" aria-hidden="true"></div>
      <RingTimer progress={timer.progress} label={timer.label} {size} />
    </div>

    <!-- Pomodoro block -->
    {#if timer.mode === 'Pomodoro' && routine}
      <div class="pomodoro">
        <div class="dots">
          {#each Array(dotCount) as _, i (i)}
            <span class="dot" class:filled={i < filledCount}></span>
          {/each}
        </div>
        <p class="pomo-label">포모도로 {timer.pomodoroIndex}</p>
        <p class="pomo-config">{routine.focus_minutes}분 집중 · {routine.break_minutes}분 휴식</p>
      </div>
    {/if}

    <!-- Today mini card -->
    {#if routine}
      <div class="today-card">
        <p class="today-text">
          오늘 {routine.name}
          <span class="mono">{formatDuration(timer.routineTodaySecs)} / {formatDuration(timer.targetSecs)}</span>
        </p>
        <div class="mini-track">
          <div class="mini-fill" style="width: {timer.progress * 100}%"></div>
        </div>
      </div>
    {/if}

    <!-- 3-button control row -->
    <div class="controls">
      <button class="circle outline" aria-label="세션 종료" onclick={handleStop}>
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
          <rect x="3" y="3" width="10" height="10" rx="2" fill="currentColor" />
        </svg>
      </button>

      <button class="circle solid" aria-label={centerLabel} onclick={handlePauseResume}>
        {#if timer.state === 'Paused'}
          <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M8 5v14l11-7z" fill="currentColor" />
          </svg>
        {:else}
          <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
            <rect x="6" y="5" width="4" height="14" rx="1.5" fill="currentColor" />
            <rect x="14" y="5" width="4" height="14" rx="1.5" fill="currentColor" />
          </svg>
        {/if}
      </button>

      {#if timer.phase === 'Break'}
        <button class="circle outline" aria-label="건너뛰기" onclick={handleSkipBreak}>
          <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M6 5v14l8-7z" fill="currentColor" />
            <rect x="15" y="5" width="3" height="14" rx="1" fill="currentColor" />
          </svg>
        </button>
      {:else}
        <button
          class="circle outline"
          aria-label="다음 제안"
          disabled={!nextRoutine}
          onclick={() => nextRoutine && handleSwitch(nextRoutine.id)}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M6 5v14l8-7z" fill="currentColor" />
            <rect x="15" y="5" width="3" height="14" rx="1" fill="currentColor" />
          </svg>
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .focus {
    position: relative;
    min-height: 100%;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 56px 24px 40px;
  }

  /* Back pill */
  .back-pill {
    position: absolute;
    top: 16px;
    left: 16px;
    padding: 6px 14px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-ui);
    cursor: pointer;
    transition: background 150ms;
  }
  .back-pill:hover {
    background: var(--row-hover);
  }

  .stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    width: 100%;
    max-width: 360px;
  }

  /* Eyebrow + routine name */
  .head {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    text-align: center;
  }
  .eyebrow {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--faint);
  }
  .routine-name {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    color: var(--ink);
    line-height: 1.2;
  }

  /* Ring + radial glow */
  .ring-stage {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .radial-glow {
    position: absolute;
    inset: -40px;
    background: radial-gradient(circle at center, var(--radial), transparent 70%);
    pointer-events: none;
    z-index: 0;
  }
  .ring-stage :global(svg) {
    position: relative;
    z-index: 1;
  }
  /* Pomodoro block */
  .pomodoro {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .dots {
    display: flex;
    gap: 7px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--dot-empty);
  }
  .dot.filled {
    background: var(--accent);
    filter: var(--ring-glow);
  }
  .pomo-label {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
  }
  .pomo-config {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }

  /* Today mini card */
  .today-card {
    width: 100%;
    background: var(--today-card);
    border-radius: var(--r-btn);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .today-text {
    margin: 0;
    font-size: 13px;
    color: var(--muted);
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
  }
  .today-text .mono {
    color: var(--faint2);
    font-size: 12px;
  }
  .mini-track {
    width: 100%;
    height: 5px;
    background: var(--track);
    border-radius: var(--r-bar);
    overflow: hidden;
  }
  .mini-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--r-bar);
    transition: width 300ms;
  }

  /* Control row */
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 24px;
    margin-top: 8px;
  }
  .circle {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    cursor: pointer;
    padding: 0;
  }
  .circle.outline {
    width: 48px;
    height: 48px;
    background: var(--card);
    border: 1px solid var(--border);
    color: var(--muted);
    transition: background 150ms;
  }
  .circle.outline:hover:not(:disabled) {
    background: var(--row-hover);
  }
  .circle.outline:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .circle.solid {
    width: 66px;
    height: 66px;
    background: var(--accent);
    border: none;
    color: #fff;
    box-shadow: var(--btn-glow);
    transition: filter 150ms;
  }
  .circle.solid:hover {
    filter: brightness(1.05);
  }
</style>
