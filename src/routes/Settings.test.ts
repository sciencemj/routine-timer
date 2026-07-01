import { describe, it, expect, beforeEach } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { resetTimer } from '$lib/timer.svelte';
import Settings from './Settings.svelte';

describe('Settings', () => {
  beforeEach(() => {
    resetTauri();
    resetTimer();
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'routines_list' ? Promise.resolve([]) :
      cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused' }) :
      cmd === 'stats_today' ? Promise.resolve({
        total_secs: 0, completed: 0, routine_count: 0,
        remaining_secs: 0, streak: 0, best_streak: 0, per_routine: {},
      }) :
      Promise.resolve()
    );
  });

  it('clicking the 다크 theme segment invokes settings_set with key theme', async () => {
    const { findByText } = render(Settings);

    await fireEvent.click(await findByText('다크'));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'settings_set',
        expect.objectContaining({ key: 'theme', value: 'dark' })
      );
    });
  });

  it('changing the streak rule invokes settings_set with key streak_rule', async () => {
    const { findByText } = render(Settings);

    await fireEvent.click(await findByText('모든 루틴 완성'));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'settings_set',
        expect.objectContaining({ key: 'streak_rule', value: 'all_completed' })
      );
    });
  });
});
