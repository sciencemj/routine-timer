import { describe, it, expect, beforeEach } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { resetTimer } from '$lib/timer.svelte';
import Settings from './Settings.svelte';

describe('Settings', () => {
  beforeEach(() => { resetTauri(); resetTimer(); invokeMock.mockImplementation((cmd: string) =>
    cmd === 'routines_list' ? Promise.resolve([]) :
    cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused' }) :
    cmd === 'stats_today' ? Promise.resolve({ total_secs:0, completed:0, routine_count:0, remaining_secs:0, streak:0, per_routine:{} }) :
    Promise.resolve()); });
  it('submitting the new-routine form invokes routine_create', async () => {
    const { findByLabelText, findByText } = render(Settings);
    await fireEvent.input(await findByLabelText('이름'), { target: { value: '독서' } });
    await fireEvent.click(await findByText('저장'));
    expect(invokeMock.mock.calls.some(c => c[0] === 'routine_create')).toBe(true);
  });
});
