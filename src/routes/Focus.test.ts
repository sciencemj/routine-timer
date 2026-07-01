import { describe, it, expect, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { resetTimer } from '$lib/timer.svelte';
import Focus from './Focus.svelte';

describe('Focus', () => {
  beforeEach(() => {
    resetTauri();
    resetTimer();
  });
  it('pause control invokes timer_pause', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'timer_get_state') {
        return Promise.resolve({ state: 'Running', mode: 'Continuous', phase: 'Focus', routine_id: 1, pomodoro_index: 1, remaining_secs: 60, session_seconds: 0, routine_today_secs: 0, target_secs: 60, state_changed: false, event: null, remaining_label: '01:00' });
      }
      if (cmd === 'routines_list') return Promise.resolve([]);
      if (cmd === 'stats_today') {
        return Promise.resolve({ total_secs: 0, completed: 0, routine_count: 0, remaining_secs: 0, streak: 0, best_streak: 0, per_routine: {} });
      }
      return Promise.resolve();
    });
    const { findByLabelText } = render(Focus);
    // Center control is icon-only — found by its accessible name.
    const btn = await findByLabelText('일시정지');
    await fireEvent.click(btn);
    expect(invokeMock).toHaveBeenCalledWith('timer_pause');
  });
});
