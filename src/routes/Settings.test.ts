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
      cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused', day_start_hour: '8' }) :
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

  it('clicking DB 리셋 opens a confirm modal, and only invokes db_reset after confirming', async () => {
    const { findByText, queryByText } = render(Settings);

    await fireEvent.click(await findByText('DB 리셋'));

    // Confirm modal appears; db_reset must NOT have been called yet.
    await findByText('정말 삭제할까요?');
    expect(invokeMock).not.toHaveBeenCalledWith('db_reset');

    await fireEvent.click(await findByText('삭제'));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('db_reset');
    });
    expect(queryByText('정말 삭제할까요?')).toBeNull();
  });
});
