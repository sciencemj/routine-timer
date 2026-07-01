<script lang="ts">
  import { routinesStore } from '$lib/routines.svelte';
  import { themeStore } from '$lib/theme.svelte';
  import { commands } from '$lib/commands';
  import { formatDurationKo } from '$lib/time';
  import type { Routine, ThemePref, StreakRule } from '$lib/types';
  import NewRoutineModal from '$lib/components/NewRoutineModal.svelte';

  // Streak-rule preference state (seeded from settingsGet on mount)
  let streakRule = $state<StreakRule>('focused');

  // Edit-routine modal state (routine CREATION lives only in the dashboard modal)
  let editOpen = $state(false);
  let editingRoutine = $state<Routine | null>(null);

  const themeOptions: { value: ThemePref; label: string }[] = [
    { value: 'system', label: '시스템' },
    { value: 'light', label: '라이트' },
    { value: 'dark', label: '다크' },
  ];

  const streakOptions: { value: StreakRule; label: string }[] = [
    { value: 'focused', label: '집중한 날' },
    { value: 'any_completed', label: '루틴 1개+ 완성' },
    { value: 'all_completed', label: '모든 루틴 완성' },
  ];

  // On mount: refresh routines, init theme, and seed current streak_rule
  $effect(() => {
    let alive = true;
    routinesStore.refresh();
    themeStore.init();
    commands.settingsGet().then((s) => {
      if (alive) streakRule = (s['streak_rule'] as StreakRule) ?? 'focused';
    });
    return () => { alive = false; };
  });

  async function selectStreakRule(rule: StreakRule) {
    streakRule = rule;
    await commands.settingsSet('streak_rule', rule);
  }

  function openEdit(routine: Routine) {
    editingRoutine = routine;
    editOpen = true;
  }

  function closeEdit() {
    editOpen = false;
    editingRoutine = null;
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

<div class="settings">
  <div class="content">
    <h1 class="page-title">설정</h1>

    <!-- 테마 -->
    <section class="card">
      <p class="section-label">테마</p>
      <div class="segmented">
        {#each themeOptions as opt (opt.value)}
          <button
            type="button"
            class="segment"
            class:active={themeStore.pref === opt.value}
            onclick={() => themeStore.setPref(opt.value)}
          >{opt.label}</button>
        {/each}
      </div>
    </section>

    <!-- 연속 규칙 -->
    <section class="card">
      <p class="section-label">연속 규칙</p>
      <div class="segmented">
        {#each streakOptions as opt (opt.value)}
          <button
            type="button"
            class="segment"
            class:active={streakRule === opt.value}
            onclick={() => selectStreakRule(opt.value)}
          >{opt.label}</button>
        {/each}
      </div>
    </section>

    <!-- 루틴 관리 -->
    <section class="card">
      <p class="section-label">루틴 관리</p>
      {#if routinesStore.list.length === 0}
        <p class="empty-state">아직 루틴이 없어요</p>
      {:else}
        <div class="routine-list">
          {#each routinesStore.list as routine, i (routine.id)}
            <div class="routine-row">
              <div class="icon-tile">{routine.icon}</div>
              <div class="row-main">
                <span class="row-name">{routine.name}</span>
                <span class="row-time">요구 시간 {formatDurationKo(routine.target_seconds)}</span>
              </div>
              <div class="row-actions">
                <button
                  type="button"
                  class="icon-btn"
                  onclick={() => moveUp(i)}
                  disabled={i === 0}
                  aria-label="위로"
                >↑</button>
                <button
                  type="button"
                  class="icon-btn"
                  onclick={() => moveDown(i)}
                  disabled={i === routinesStore.list.length - 1}
                  aria-label="아래로"
                >↓</button>
                <button type="button" class="text-btn" onclick={() => openEdit(routine)}>편집</button>
                <button type="button" class="text-btn danger" onclick={() => deleteRoutine(routine.id)}>삭제</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</div>

<NewRoutineModal open={editOpen} editRoutine={editingRoutine} onclose={closeEdit} />

<style>
  .settings {
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

  .page-title {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    color: var(--ink);
  }

  .card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .section-label {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  /* Segmented control */
  .segmented {
    display: flex;
    background: var(--track);
    border-radius: 11px;
    padding: 3px;
    gap: 2px;
  }
  .segment {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--faint);
    font-family: var(--font-ui);
    font-weight: 600;
    font-size: 12.5px;
    padding: 8px 6px;
    border-radius: 8px;
    cursor: pointer;
    transition: color 0.15s, background 0.15s, box-shadow 0.15s;
    white-space: nowrap;
  }
  .segment.active {
    background: var(--card);
    color: var(--ink);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1), 0 0 0 0.5px rgba(0, 0, 0, 0.06);
  }

  /* Routine management */
  .empty-state {
    margin: 0;
    padding: 12px 0;
    font-size: 13px;
    color: var(--faint);
  }
  .routine-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .routine-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--hair);
  }
  .routine-row:last-child {
    border-bottom: none;
    padding-bottom: 0;
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
    gap: 2px;
    min-width: 0;
  }
  .row-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--ink);
  }
  .row-time {
    font-size: 12px;
    color: var(--faint2);
  }
  .row-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
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
  .text-btn {
    border: none;
    background: none;
    color: var(--accent);
    font-family: var(--font-ui);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    padding: 4px 4px;
  }
  .text-btn.danger {
    color: #e5484d;
  }
</style>
