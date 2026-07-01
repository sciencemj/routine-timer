<script lang="ts">
  import { themeStore } from '$lib/theme.svelte';
  import { commands } from '$lib/commands';
  import type { ThemePref, StreakRule } from '$lib/types';

  // Streak-rule preference state (seeded from settingsGet on mount)
  let streakRule = $state<StreakRule>('focused');

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

  // On mount: init theme, and seed current streak_rule
  $effect(() => {
    let alive = true;
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
  </div>
</div>

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
</style>
