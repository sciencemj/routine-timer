<script lang="ts">
  import { routinesStore } from '$lib/routines.svelte';
  import { commands } from '$lib/commands';
  import { formatDurationKo } from '$lib/time';

  let { open, onclose }: {
    open: boolean;
    onclose: () => void;
  } = $props();

  // Form state — pomodoro defaults ON per spec
  let name = $state('');
  let selectedEmoji = $state('🎯');
  let targetSeconds = $state(3600);
  let pomodoroEnabled = $state(true);

  const emojis = ['🎯', '📚', '✍️', '🏃', '🧘', '🌐'];

  const chips: { label: string; secs: number }[] = [
    { label: '30분',      secs: 1800 },
    { label: '45분',      secs: 2700 },
    { label: '1시간',     secs: 3600 },
    { label: '1시간 30분', secs: 5400 },
    { label: '2시간',     secs: 7200 },
  ];

  function resetForm() {
    name = '';
    selectedEmoji = '🎯';
    targetSeconds = 3600;
    pomodoroEnabled = true;
  }

  async function handleCreate() {
    if (!name.trim()) return;
    await commands.routineCreate({
      name: name.trim(),
      icon: selectedEmoji,
      color: null,
      target_seconds: targetSeconds,
      pomodoro_enabled: pomodoroEnabled,
      focus_minutes: 25,
      break_minutes: 5,
    });
    await routinesStore.refresh();
    resetForm();
    onclose();
  }
</script>

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="새 루틴">
    <div class="modal-header">
      <button class="close-btn" onclick={onclose}>취소</button>
      <h2 class="modal-title">새 루틴</h2>
      <div class="header-spacer"></div>
    </div>

    <div class="modal-body">
      <!-- 이름 -->
      <div class="field">
        <label class="field-label" for="routine-name">이름</label>
        <input
          id="routine-name"
          class="text-input"
          type="text"
          placeholder="루틴 이름"
          bind:value={name}
        />
      </div>

      <!-- 아이콘 -->
      <div class="field">
        <span class="field-label">아이콘</span>
        <div class="emoji-grid">
          {#each emojis as emoji}
            <button
              class="emoji-btn"
              class:selected={selectedEmoji === emoji}
              onclick={() => { selectedEmoji = emoji; }}
              aria-label={emoji}
              aria-pressed={selectedEmoji === emoji}
            >{emoji}</button>
          {/each}
        </div>
      </div>

      <!-- 요구 시간 -->
      <div class="field">
        <span class="field-label">요구 시간</span>
        <p class="duration-display">{formatDurationKo(targetSeconds)}</p>
        <div class="chips">
          {#each chips as chip}
            <button
              class="chip"
              class:selected={targetSeconds === chip.secs}
              onclick={() => { targetSeconds = chip.secs; }}
            >{chip.label}</button>
          {/each}
        </div>
      </div>

      <!-- 포모도로 -->
      <div class="field">
        <div class="toggle-row">
          <div class="toggle-info">
            <span class="field-label">포모도로</span>
            <span class="toggle-sub">25분 집중 · 5분 휴식</span>
          </div>
          <button
            class="toggle"
            class:on={pomodoroEnabled}
            onclick={() => { pomodoroEnabled = !pomodoroEnabled; }}
            role="switch"
            aria-checked={pomodoroEnabled}
            aria-label="포모도로"
          >
            <span class="toggle-knob"></span>
          </button>
        </div>
      </div>
    </div>

    <div class="modal-footer">
      <button class="btn-outline" onclick={onclose}>취소</button>
      <button
        class="btn-accent"
        onclick={handleCreate}
        disabled={!name.trim()}
      >루틴 추가</button>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    z-index: 100;
  }
  .modal-header {
    display: flex;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--hair);
  }
  .close-btn {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    padding: 0;
    font-family: var(--font-ui);
    min-width: 40px;
    text-align: left;
  }
  .modal-title {
    flex: 1;
    text-align: center;
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
    margin: 0;
  }
  .header-spacer {
    min-width: 40px;
  }
  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .field-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
  }
  .text-input {
    background: var(--today-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 12px 14px;
    font-size: 14px;
    color: var(--ink);
    font-family: var(--font-ui);
    outline: none;
    width: 100%;
  }
  .text-input::placeholder {
    color: var(--faint2);
  }
  .text-input:focus {
    border-color: var(--accent);
  }
  .emoji-grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 8px;
  }
  .emoji-btn {
    aspect-ratio: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    border: 2px solid transparent;
    border-radius: 12px;
    background: var(--today-card);
    cursor: pointer;
    transition: background 150ms;
    padding: 0;
  }
  .emoji-btn.selected {
    background: var(--accent-bg);
    border-color: var(--accent);
  }
  .duration-display {
    font-size: 27px;
    font-weight: 600;
    color: var(--ink);
    margin: 0;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chip {
    padding: 6px 14px;
    border-radius: var(--r-chip);
    border: 1px solid var(--border);
    background: var(--today-card);
    color: var(--muted);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    font-family: var(--font-ui);
    transition: background 150ms, color 150ms;
  }
  .chip.selected {
    background: var(--accent-bg);
    color: var(--accent);
    border-color: transparent;
    font-weight: 600;
  }
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .toggle-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .toggle-sub {
    font-size: 12px;
    color: var(--faint);
  }
  .toggle {
    width: 48px;
    height: 28px;
    border-radius: 14px;
    border: none;
    background: var(--border);
    cursor: pointer;
    position: relative;
    transition: background 200ms;
    padding: 0;
    flex-shrink: 0;
  }
  .toggle.on {
    background: var(--accent);
  }
  .toggle-knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: white;
    transition: left 200ms;
    pointer-events: none;
  }
  .toggle.on .toggle-knob {
    left: 23px;
  }
  .modal-footer {
    display: flex;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--hair);
  }
  .btn-outline {
    flex: 1;
    padding: 13px;
    border: 1px solid var(--border);
    border-radius: var(--r-btn);
    background: transparent;
    color: var(--ink);
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    font-family: var(--font-ui);
  }
  .btn-accent {
    flex: 1;
    padding: 13px;
    border: none;
    border-radius: var(--r-btn);
    background: var(--accent);
    color: white;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    font-family: var(--font-ui);
    transition: opacity 150ms;
  }
  .btn-accent:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
