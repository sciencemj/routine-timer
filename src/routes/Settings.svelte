<script lang="ts">
  import { themeStore } from '$lib/theme.svelte';
  import { commands } from '$lib/commands';
  import { routinesStore } from '$lib/routines.svelte';
  import type { ThemePref, StreakRule } from '$lib/types';

  // Streak-rule preference state (seeded from settingsGet on mount)
  let streakRule = $state<StreakRule>('focused');
  // Day-start-hour preference state (seeded from settingsGet on mount)
  let dayStartHour = $state('8');
  // "DB 리셋" destructive-confirm modal
  let confirmOpen = $state(false);
  let deleteBtn = $state<HTMLButtonElement | null>(null);

  // Focus the destructive action when the confirm modal opens, so keyboard
  // users land somewhere sensible (still requires an explicit Enter to confirm).
  $effect(() => {
    if (confirmOpen) deleteBtn?.focus();
  });

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

  const dayStartHourOptions: { value: string; label: string }[] = Array.from(
    { length: 24 },
    (_, h) => ({ value: String(h), label: `${h}시` })
  );

  // On mount: init theme, and seed current streak_rule + day_start_hour
  $effect(() => {
    let alive = true;
    themeStore.init();
    commands.settingsGet().then((s) => {
      if (!alive) return;
      streakRule = (s['streak_rule'] as StreakRule) ?? 'focused';
      dayStartHour = s['day_start_hour'] ?? '8';
    });
    return () => { alive = false; };
  });

  async function selectStreakRule(rule: StreakRule) {
    streakRule = rule;
    await commands.settingsSet('streak_rule', rule);
  }

  async function selectDayStartHour(value: string) {
    dayStartHour = value;
    await commands.settingsSet('day_start_hour', value);
  }

  function openConfirm() { confirmOpen = true; }
  function closeConfirm() { confirmOpen = false; }

  async function confirmReset() {
    await commands.dbReset();
    await routinesStore.refresh();
    closeConfirm();
  }

  function onConfirmKeydown(e: KeyboardEvent) {
    if (confirmOpen && e.key === 'Escape') closeConfirm();
  }
</script>

<svelte:window onkeydown={onConfirmKeydown} />

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

    <!-- 하루 시작 시각 -->
    <section class="card">
      <p class="section-label">하루 시작 시각</p>
      <select
        class="hour-select"
        value={dayStartHour}
        onchange={(e) => selectDayStartHour(e.currentTarget.value)}
      >
        {#each dayStartHourOptions as opt (opt.value)}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
      <p class="setting-caption">이 시각을 기준으로 하루가 바뀝니다 (자정~이 시각의 집중은 전날로 집계)</p>
    </section>

    <!-- 데이터 (danger zone) -->
    <section class="card danger-card">
      <p class="section-label danger-label">데이터</p>
      <p class="setting-caption">모든 루틴과 집중 기록을 삭제합니다. 되돌릴 수 없습니다.</p>
      <button type="button" class="danger-btn" onclick={openConfirm}>DB 리셋</button>
    </section>
  </div>
</div>

{#if confirmOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="confirm-backdrop"
    role="presentation"
    onclick={closeConfirm}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="confirm-card"
      role="alertdialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="confirm-title"
      onclick={(e) => e.stopPropagation()}
    >
      <h2 id="confirm-title" class="confirm-title">정말 삭제할까요?</h2>
      <p class="confirm-body">루틴과 집중 기록이 모두 사라지며 되돌릴 수 없습니다.</p>
      <div class="confirm-actions">
        <button type="button" class="btn-outline" onclick={closeConfirm}>취소</button>
        <button type="button" class="btn-danger" bind:this={deleteBtn} onclick={confirmReset}>삭제</button>
      </div>
    </div>
  </div>
{/if}

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

  /* Day-start-hour select */
  .hour-select {
    width: 100%;
    background: var(--track);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--ink);
    font-family: var(--font-ui);
    font-weight: 600;
    font-size: 13px;
    padding: 8px 10px;
    cursor: pointer;
  }
  .setting-caption {
    margin: 0;
    font-size: 12px;
    color: var(--faint);
  }

  /* 데이터 (danger zone) */
  .danger-card {
    border-color: color-mix(in srgb, var(--neg) 30%, var(--border));
  }
  .danger-label {
    color: var(--neg);
  }
  .danger-btn {
    align-self: flex-start;
    border: none;
    border-radius: var(--r-btn);
    background: var(--neg);
    color: white;
    font-family: var(--font-ui);
    font-weight: 600;
    font-size: 13px;
    padding: 10px 18px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .danger-btn:hover {
    opacity: 0.9;
  }

  /* Confirm modal */
  .confirm-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
    padding: 20px;
  }
  .confirm-card {
    width: 100%;
    max-width: 340px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    padding: 22px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.28);
  }
  .confirm-title {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
    color: var(--ink);
  }
  .confirm-body {
    margin: 0;
    font-size: 13px;
    color: var(--muted);
    line-height: 1.5;
  }
  .confirm-actions {
    display: flex;
    gap: 10px;
    margin-top: 10px;
  }
  .confirm-actions .btn-outline {
    flex: 1;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--r-btn);
    background: transparent;
    color: var(--ink);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    font-family: var(--font-ui);
  }
  .btn-danger {
    flex: 1;
    padding: 12px;
    border: none;
    border-radius: var(--r-btn);
    background: var(--neg);
    color: white;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    font-family: var(--font-ui);
    transition: opacity 0.15s;
  }
  .btn-danger:hover {
    opacity: 0.9;
  }
  .btn-danger:focus-visible {
    outline: 2px solid var(--neg);
    outline-offset: 2px;
  }
</style>
