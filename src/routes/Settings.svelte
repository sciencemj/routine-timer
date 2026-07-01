<script lang="ts">
  import { routinesStore } from '$lib/routines.svelte';
  import { themeStore } from '$lib/theme.svelte';
  import { commands } from '$lib/commands';
  import type { Routine, NewRoutine, ThemePref, StreakRule } from '$lib/types';

  // Form state
  let editingId = $state<number | null>(null);
  let name = $state('');
  let icon = $state('');
  let hours = $state(0);
  let minutes = $state(0);
  let pomodoroEnabled = $state(false);
  let focusMinutes = $state(25);
  let breakMinutes = $state(5);

  // Preferences state
  let streakRule = $state<StreakRule>('focused');

  // On mount: refresh routines, init theme, and load current streak_rule
  $effect(() => {
    routinesStore.refresh();
    themeStore.init();

    let alive = true;
    commands.settingsGet().then(s => {
      if (alive) streakRule = (s['streak_rule'] as StreakRule) ?? 'focused';
    });
    return () => { alive = false; };
  });

  function editRoutine(r: Routine) {
    editingId = r.id;
    name = r.name;
    icon = r.icon;
    hours = Math.floor(r.target_seconds / 3600);
    minutes = Math.floor((r.target_seconds % 3600) / 60);
    pomodoroEnabled = r.pomodoro_enabled;
    focusMinutes = r.focus_minutes;
    breakMinutes = r.break_minutes;
  }

  function resetForm() {
    editingId = null;
    name = '';
    icon = '';
    hours = 0;
    minutes = 0;
    pomodoroEnabled = false;
    focusMinutes = 25;
    breakMinutes = 5;
  }

  async function save() {
    const target_seconds = hours * 3600 + minutes * 60;
    if (editingId !== null) {
      const existing = routinesStore.list.find(r => r.id === editingId);
      if (existing) {
        await commands.routineUpdate({
          ...existing,
          name,
          icon,
          target_seconds,
          pomodoro_enabled: pomodoroEnabled,
          focus_minutes: focusMinutes,
          break_minutes: breakMinutes,
        });
      }
    } else {
      const newRoutine: NewRoutine = {
        name,
        icon,
        color: null,
        target_seconds,
        pomodoro_enabled: pomodoroEnabled,
        focus_minutes: focusMinutes,
        break_minutes: breakMinutes,
      };
      await commands.routineCreate(newRoutine);
    }
    await routinesStore.refresh();
    resetForm();
  }

  async function deleteRoutine(id: number) {
    await commands.routineDelete(id);
    await routinesStore.refresh();
  }

  async function moveUp(index: number) {
    if (index <= 0) return;
    const newList = [...routinesStore.list];
    [newList[index - 1], newList[index]] = [newList[index], newList[index - 1]];
    await commands.routineReorder(newList.map(r => r.id));
    await routinesStore.refresh();
  }

  async function moveDown(index: number) {
    if (index >= routinesStore.list.length - 1) return;
    const newList = [...routinesStore.list];
    [newList[index], newList[index + 1]] = [newList[index + 1], newList[index]];
    await commands.routineReorder(newList.map(r => r.id));
    await routinesStore.refresh();
  }

  async function handleStreakChange(e: Event) {
    const value = (e.target as HTMLSelectElement).value as StreakRule;
    streakRule = value;
    await commands.settingsSet('streak_rule', value);
  }
</script>

<div class="settings">
  <h1>설정</h1>

  <!-- ── Routine Editor ── -->
  <section class="section">
    <h2>루틴 편집</h2>

    {#each routinesStore.list as routine, i (routine.id)}
      <div class="routine-item">
        <span class="routine-name">{routine.icon} {routine.name}</span>
        <div class="routine-actions">
          <button type="button" onclick={() => moveUp(i)} disabled={i === 0} aria-label="위로">↑</button>
          <button type="button" onclick={() => moveDown(i)} disabled={i === routinesStore.list.length - 1} aria-label="아래로">↓</button>
          <button type="button" onclick={() => editRoutine(routine)}>편집</button>
          <button type="button" onclick={() => deleteRoutine(routine.id)}>삭제</button>
        </div>
      </div>
    {/each}

    <form onsubmit={(e) => { e.preventDefault(); save(); }}>
      <h3>{editingId !== null ? '루틴 수정' : '새 루틴'}</h3>

      <div class="field">
        <label>이름
          <input type="text" bind:value={name} />
        </label>
      </div>

      <div class="field">
        <label>아이콘
          <input type="text" bind:value={icon} />
        </label>
      </div>

      <div class="field">
        <label>요구 시간
          <span class="time-inputs">
            <input type="number" min="0" bind:value={hours} aria-label="시간" /> 시간
            <input type="number" min="0" max="59" bind:value={minutes} aria-label="분" /> 분
          </span>
        </label>
      </div>

      <div class="field">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={pomodoroEnabled} />
          포모도로
        </label>
      </div>

      {#if pomodoroEnabled}
        <div class="field">
          <label>집중 분
            <input type="number" min="1" bind:value={focusMinutes} />
          </label>
        </div>
        <div class="field">
          <label>휴식 분
            <input type="number" min="1" bind:value={breakMinutes} />
          </label>
        </div>
      {/if}

      <div class="form-actions">
        <button type="submit">저장</button>
        {#if editingId !== null}
          <button type="button" onclick={resetForm}>취소</button>
        {/if}
      </div>
    </form>
  </section>

  <!-- ── Preferences ── -->
  <section class="section">
    <h2>환경 설정</h2>

    <div class="field">
      <p class="field-label">테마</p>
      <div class="radio-group">
        <label>
          <input
            type="radio"
            name="theme"
            value="system"
            checked={themeStore.pref === 'system'}
            onchange={() => themeStore.setPref('system')}
          />
          시스템
        </label>
        <label>
          <input
            type="radio"
            name="theme"
            value="light"
            checked={themeStore.pref === 'light'}
            onchange={() => themeStore.setPref('light')}
          />
          라이트
        </label>
        <label>
          <input
            type="radio"
            name="theme"
            value="dark"
            checked={themeStore.pref === 'dark'}
            onchange={() => themeStore.setPref('dark')}
          />
          다크
        </label>
      </div>
    </div>

    <div class="field">
      <label>연속 달성 기준
        <select value={streakRule} onchange={handleStreakChange}>
          <option value="focused">집중한 날</option>
          <option value="any_completed">루틴 1개+ 완성</option>
          <option value="all_completed">모든 루틴 완성</option>
        </select>
      </label>
    </div>
  </section>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding: 24px;
  }

  h1 {
    margin: 0;
    font-size: 1.6rem;
    font-weight: 700;
    color: var(--text);
  }

  h2 {
    margin: 0 0 8px;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text);
  }

  h3 {
    margin: 0 0 8px;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text);
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .routine-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 0;
    border-bottom: 1px solid var(--border, #eee);
  }

  .routine-name {
    font-size: 0.95rem;
    color: var(--text);
  }

  .routine-actions {
    display: flex;
    gap: 4px;
  }

  .routine-actions button {
    padding: 4px 8px;
    font-size: 0.8rem;
    cursor: pointer;
    border: 1px solid var(--border, #ddd);
    border-radius: 4px;
    background: var(--surface, #fff);
    color: var(--text);
  }

  .routine-actions button:disabled {
    opacity: 0.3;
    cursor: default;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    border: 1px solid var(--border, #eee);
    border-radius: 8px;
    background: var(--surface, #fafafa);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.9rem;
    color: var(--text);
  }

  .checkbox-label {
    flex-direction: row !important;
    align-items: center;
    gap: 8px !important;
  }

  .time-inputs {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .field input[type='text'],
  .field input[type='number'],
  .field select {
    padding: 6px 8px;
    border: 1px solid var(--border, #ddd);
    border-radius: 4px;
    font-size: 0.9rem;
    background: var(--bg, #fff);
    color: var(--text);
  }

  .field input[type='number'] {
    width: 60px;
  }

  .form-actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .form-actions button[type='submit'] {
    padding: 8px 20px;
    background: var(--accent, #4f6ef7);
    color: #fff;
    border: none;
    border-radius: 6px;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .form-actions button[type='button'] {
    padding: 8px 16px;
    background: var(--surface, #eee);
    color: var(--text);
    border: 1px solid var(--border, #ddd);
    border-radius: 6px;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .field-label {
    margin: 0 0 4px;
    font-size: 0.9rem;
    font-weight: 500;
    color: var(--text);
  }

  .radio-group {
    display: flex;
    gap: 16px;
  }

  .radio-group label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.9rem;
    color: var(--text);
    cursor: pointer;
  }
</style>
