<script lang="ts">
  import { push } from 'svelte-spa-router';
  import { timer } from '$lib/timer.svelte';
  import { routinesStore, initRoutinesListeners } from '$lib/routines.svelte';
  import { formatDuration, formatClock } from '$lib/time';
  import { commands } from '$lib/commands';
  import RoutineCard from '$lib/components/RoutineCard.svelte';

  // Live clock — refreshed every minute
  let now = $state(new Date());
  $effect(() => {
    const id = setInterval(() => { now = new Date(); }, 60_000);
    return () => clearInterval(id);
  });

  // Greeting based on current hour — derives from reactive `now` so it flips at midnight
  const greeting = $derived(now.getHours() < 12 ? '좋은 아침이에요' : '좋은 오후예요');

  // Convenience alias
  const stats = $derived(routinesStore.stats);

  // On mount: load routines + subscribe to change events
  $effect(() => {
    let alive = true;
    let cleanup: (() => void) | undefined;
    routinesStore.refresh();
    initRoutinesListeners().then((fn) => { if (alive) cleanup = fn; else fn(); });
    return () => { alive = false; cleanup?.(); };
  });

  async function start(id: number) {
    await commands.timerStart(id);
    push('/focus');
  }
</script>

<div class="home">
  <header>
    <p class="greeting">{greeting}</p>
    <p class="clock">{formatClock(now)}</p>
  </header>

  <section class="summary">
    {#if stats}
      <p class="summary-bar">남은 {formatDuration(stats.remaining_secs)} · {stats.routine_count}개 루틴 중 {stats.completed}개 완료</p>
      <p class="daily-focus">하루 집중 {formatDuration(stats.total_secs)}</p>
      <p class="streak">연속 {stats.streak}일 · 최고 {stats.best_streak}일</p>
    {/if}
  </section>

  <section class="routines">
    {#each routinesStore.list as routine (routine.id)}
      <RoutineCard
        {routine}
        todaySecs={routinesStore.secondsFor(routine.id)}
        active={timer.routineId === routine.id && timer.isActive}
        onclick={() => start(routine.id)}
      />
    {/each}
  </section>

  <button class="new-routine" onclick={() => push('/settings')}>새 루틴</button>
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
  }

  header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .greeting {
    font-size: 1.4rem;
    font-weight: 700;
    margin: 0;
    color: var(--text);
  }

  .clock {
    font-size: 1rem;
    margin: 0;
    color: var(--muted);
  }

  .summary {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.9rem;
    color: var(--text);
  }

  .summary p {
    margin: 0;
  }

  .routines {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
  }

  .new-routine {
    align-self: flex-start;
    padding: 8px 20px;
    background: var(--accent, #4f6ef7);
    color: #fff;
    border: none;
    border-radius: 8px;
    font-size: 0.9rem;
    cursor: pointer;
  }
</style>
