import { describe, it, expect, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { resetTimer } from '$lib/timer.svelte';
import type { Routine } from '$lib/types';
import Home from './Home.svelte';

const routine: Routine = {
  id: 1, name: '독서', icon: '📚', color: null,
  target_seconds: 1800, pomodoro_enabled: false,
  focus_minutes: 25, break_minutes: 5,
  sort_order: 1, archived: false, created_at: '',
};

describe('Home', () => {
  beforeEach(() => {
    resetTauri();
    resetTimer();
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'routines_list' ? Promise.resolve([]) :
      cmd === 'stats_today' ? Promise.resolve({ total_secs: 0, completed: 0, routine_count: 0, remaining_secs: 0, streak: 0, best_streak: 0, per_routine: {} }) :
      Promise.resolve());
  });
  it('renders a greeting', async () => {
    const { findByText } = render(Home);
    expect(await findByText(/좋은 (아침이에요|오후예요)/)).toBeInTheDocument();
  });

  it('toggling 수정 shows edit controls, and clicking a row in edit mode opens the edit modal', async () => {
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'routines_list' ? Promise.resolve([routine]) :
      cmd === 'stats_today' ? Promise.resolve({ total_secs: 0, completed: 0, routine_count: 1, remaining_secs: 1800, streak: 0, best_streak: 0, per_routine: {} }) :
      Promise.resolve());

    const { findByText, getByLabelText, queryByLabelText } = render(Home);

    await findByText('독서');
    expect(queryByLabelText('삭제')).toBeNull();

    await fireEvent.click(await findByText('수정'));

    const deleteBtn = await findByText('완료') && getByLabelText('삭제');
    expect(deleteBtn).toBeInTheDocument();

    await fireEvent.click(await findByText('독서'));

    expect(await findByText('루틴 편집')).toBeInTheDocument();
  });
});
