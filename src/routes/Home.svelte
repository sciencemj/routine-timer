<script lang="ts">
  import { push } from 'svelte-spa-router';
  import { timer } from '$lib/timer.svelte';
  import { routinesStore, initRoutinesListeners } from '$lib/routines.svelte';
  import { formatDurationKo } from '$lib/time';
  import { commands } from '$lib/commands';
  import type { Routine } from '$lib/types';
  import RoutineRow from '$lib/components/RoutineRow.svelte';
  import NewRoutineModal from '$lib/components/NewRoutineModal.svelte';

  const DAY_NAMES = ['일요일', '월요일', '화요일', '수요일', '목요일', '금요일', '토요일'];

  // Live clock — refreshed every minute
  let now = $state(new Date());
  $effect(() => {
    const id = setInterval(() => { now = new Date(); }, 60_000);
    return () => clearInterval(id);
  });

  const dateEyebrow = $derived(`${now.getMonth() + 1}월 ${now.getDate()}일 ${DAY_NAMES[now.getDay()]}`);
  const greeting = $derived(now.getHours() < 12 ? '좋은 아침이에요.' : '좋은 오후예요.');

  const stats = $derived(routinesStore.stats);
  const totalTarget = $derived(routinesStore.list.reduce((acc, r) => acc + r.target_seconds, 0));
  const overallPercent = $derived(
    stats && totalTarget > 0 ? Math.min(1, stats.total_secs / totalTarget) : 0
  );

  // SVG ring constants
  const RING_R = 32;
  const CIRCUMFERENCE = 2 * Math.PI * RING_R; // ≈ 201.06

  // Modal state
  let showNew = $state(false);
  let editRoutine = $state<Routine | null>(null);

  // Edit-mode toggle for the routine list
  let editing = $state(false);

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

  function openCreate() {
    editRoutine = null;
    showNew = true;
  }

  function openEdit(routine: Routine) {
    editRoutine = routine;
    showNew = true;
  }

  function closeModal() {
    showNew = false;
    editRoutine = null;
  }

  async function deleteRoutine(id: number) {
    await commands.routineDelete(id);
    await routinesStore.refresh();
  }

  async function moveUp(index: number) {
    if (index <= 0) return;
    const newList = [...routinesStore.list];
    [newList[index - 1], newList[index]] = [newList[index], newList[index - 1]];
    await commands.routineReorder(newList.map((r) => r.id));
    await routinesStore.refresh();
  }

  async function moveDown(index: number) {
    if (index >= routinesStore.list.length - 1) return;
    const newList = [...routinesStore.list];
    [newList[index], newList[index + 1]] = [newList[index + 1], newList[index]];
    await commands.routineReorder(newList.map((r) => r.id));
    await routinesStore.refresh();
  }
</script>

<div class="home">
  <div class="content">
    <!-- Date eyebrow -->
    <p class="date-eyebrow">{dateEyebrow}</p>

    <!-- Greeting -->
    <h1 class="greeting">{greeting}</h1>

    <!-- Subtext + goal card (stats guard) -->
    {#if stats}
      <p class="subtext">오늘 <span class="accent">{formatDurationKo(stats.total_secs)}</span> 집중했어요.</p>

      <!-- 오늘의 목표 card -->
      <div class="goal-card">
        <!-- SVG ring -->
        <svg width="80" height="80" class="ring-svg" aria-hidden="true">
          <!-- Track -->
          <circle
            cx="40" cy="40" r={RING_R}
            fill="none"
            stroke="var(--ring-track)"
            stroke-width="9"
          />
          <!-- Progress arc -->
          <circle
            cx="40" cy="40" r={RING_R}
            fill="none"
            stroke="var(--accent)"
            stroke-width="9"
            stroke-linecap="round"
            stroke-dasharray={CIRCUMFERENCE}
            stroke-dashoffset={CIRCUMFERENCE * (1 - overallPercent)}
            transform="rotate(-90 40 40)"
          />
          <!-- Center % badge -->
          <text
            x="40" y="40"
            text-anchor="middle"
            dominant-baseline="central"
            font-size="13"
            font-weight="700"
            fill="var(--ink)"
            font-family="var(--font-ui)"
          >{Math.round(overallPercent * 100)}%</text>
        </svg>

        <!-- Right info -->
        <div class="goal-info">
          <p class="goal-label">오늘의 목표</p>
          <p class="goal-numbers">
            <span class="goal-done">{formatDurationKo(stats.total_secs)}</span><!--
            --><span class="goal-target"> / {formatDurationKo(totalTarget)}</span>
          </p>
          <div class="h-bar-track">
            <div class="h-bar-fill" style="width: {overallPercent * 100}%"></div>
          </div>
          <p class="goal-caption">남은 {formatDurationKo(stats.remaining_secs)} · {stats.routine_count}개 루틴 중 {stats.completed}개 완료</p>
        </div>
      </div>
    {/if}

    <!-- 오늘의 루틴 section -->
    <div class="routines-section">
      <div class="section-header">
        <h2 class="section-title">오늘의 루틴</h2>
        <div class="header-actions">
          <button class="edit-pill" onclick={() => { editing = !editing; }}>{editing ? '완료' : '수정'}</button>
          <button class="new-pill" onclick={openCreate}>+ 새 루틴</button>
        </div>
      </div>

      <div class="routine-list">
        {#if routinesStore.list.length === 0}
          <p class="empty-state">아직 루틴이 없어요 — 새 루틴을 추가하세요</p>
        {:else}
          {#each routinesStore.list as routine, i (routine.id)}
            <RoutineRow
              {routine}
              todaySecs={routinesStore.secondsFor(routine.id)}
              active={timer.routineId === routine.id && timer.isActive}
              onclick={() => start(routine.id)}
              {editing}
              onEdit={() => openEdit(routine)}
              onDelete={() => deleteRoutine(routine.id)}
              onMoveUp={() => moveUp(i)}
              onMoveDown={() => moveDown(i)}
              canMoveUp={i > 0}
              canMoveDown={i < routinesStore.list.length - 1}
            />
          {/each}
        {/if}
      </div>
    </div>
  </div>
</div>

<NewRoutineModal open={showNew} editRoutine={editRoutine} onclose={closeModal} />

<style>
  .home {
    min-height: 100%;
    background: var(--bg);
    display: flex;
    justify-content: center;
  }
  .content {
    width: 100%;
    max-width: 544px;
    padding: 28px 28px 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Date eyebrow */
  .date-eyebrow {
    margin: 0;
    font-size: 11px;
    font-weight: 500;
    color: var(--faint);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  /* Greeting */
  .greeting {
    margin: 0;
    font-size: 25px;
    font-weight: 600;
    color: var(--ink);
    line-height: 1.2;
  }

  /* Subtext */
  .subtext {
    margin: 0;
    font-size: 14px;
    color: var(--muted);
  }
  .accent {
    color: var(--accent);
    font-weight: 500;
  }

  /* Goal card */
  .goal-card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    padding: 20px;
    display: flex;
    align-items: center;
    gap: 20px;
  }
  .ring-svg {
    flex-shrink: 0;
  }
  .goal-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .goal-label {
    margin: 0;
    font-size: 12px;
    color: var(--faint);
  }
  .goal-numbers {
    margin: 0;
    font-size: 20px;
    line-height: 1;
  }
  .goal-done {
    font-weight: 700;
    color: var(--ink);
  }
  .goal-target {
    font-size: 15px;
    color: var(--faint2);
  }
  .h-bar-track {
    width: 100%;
    height: 5px;
    background: var(--track);
    border-radius: var(--r-bar);
    overflow: hidden;
  }
  .h-bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: var(--r-bar);
    transition: width 300ms;
  }
  .goal-caption {
    margin: 0;
    font-size: 12px;
    color: var(--faint);
  }

  /* Routines section */
  .routines-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .section-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--ink);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .new-pill {
    padding: 5px 14px;
    background: var(--accent-bg);
    color: var(--accent);
    border: none;
    border-radius: var(--r-pill);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    font-family: var(--font-ui);
    transition: opacity 150ms;
  }
  .new-pill:hover {
    opacity: 0.8;
  }
  .edit-pill {
    padding: 5px 14px;
    background: var(--track);
    color: var(--muted);
    border: none;
    border-radius: var(--r-pill);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    font-family: var(--font-ui);
    transition: opacity 150ms;
  }
  .edit-pill:hover {
    opacity: 0.8;
  }
  .routine-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .empty-state {
    margin: 0;
    padding: 20px;
    text-align: center;
    font-size: 14px;
    color: var(--faint);
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
  }
</style>
