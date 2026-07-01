import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { resetTimer } from '$lib/timer.svelte';
import Home from './Home.svelte';

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
});
