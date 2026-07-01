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
  it('pause button invokes timer_pause', async () => {
    invokeMock.mockImplementation((cmd: string) => cmd === 'timer_get_state'
      ? Promise.resolve({ state: 'Running', mode: 'Continuous', phase: 'Focus', routine_id: 1, pomodoro_index: 1, remaining_secs: 60, session_seconds: 0, routine_today_secs: 0, target_secs: 60, state_changed: false, event: null, remaining_label: '01:00' })
      : Promise.resolve());
    const { findByText } = render(Focus);
    const btn = await findByText('일시정지');
    await fireEvent.click(btn);
    expect(invokeMock).toHaveBeenCalledWith('timer_pause');
  });
});
