import { describe, it, expect, beforeEach } from 'vitest';
import { render, fireEvent, waitFor } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../../test/tauri-mock';
import { resetTimer } from '$lib/timer.svelte';
import NewRoutineModal from './NewRoutineModal.svelte';

describe('NewRoutineModal', () => {
  beforeEach(() => {
    resetTauri();
    resetTimer();
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'routines_list'
        ? Promise.resolve([])
        : cmd === 'stats_today'
          ? Promise.resolve({
              total_secs: 0, completed: 0, routine_count: 0,
              remaining_secs: 0, streak: 0, best_streak: 0, per_routine: {},
            })
          : cmd === 'routine_create'
            ? Promise.resolve({
                id: 1, name: 'Test', icon: '🎯', color: null,
                target_seconds: 3600, pomodoro_enabled: true,
                focus_minutes: 25, break_minutes: 5,
                sort_order: 1, archived: false, created_at: '',
              })
            : Promise.resolve()
    );
  });

  it('calls routine_create with pomodoro_enabled true by default', async () => {
    const { getByPlaceholderText, getByText } = render(NewRoutineModal, {
      props: { open: true, onclose: () => {} },
    });

    // Fill the name input
    const input = getByPlaceholderText('루틴 이름') as HTMLInputElement;
    input.value = '딥워크';
    await fireEvent.input(input);

    // Click 루틴 추가
    const createBtn = getByText('루틴 추가');
    await fireEvent.click(createBtn);

    // Assert routine_create was called with pomodoro_enabled: true
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'routine_create',
        expect.objectContaining({
          new: expect.objectContaining({ pomodoro_enabled: true }),
        })
      );
    });
  });

  it('renders with pomodoro toggle in ON state by default', () => {
    const { getByRole } = render(NewRoutineModal, {
      props: { open: true, onclose: () => {} },
    });
    const toggle = getByRole('switch', { name: '포모도로' });
    expect(toggle).toHaveAttribute('aria-checked', 'true');
  });

  it('disables 루틴 추가 button when name is empty', () => {
    const { getByText } = render(NewRoutineModal, {
      props: { open: true, onclose: () => {} },
    });
    const btn = getByText('루틴 추가') as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
  });

  it('does not render when open is false', () => {
    const { queryByText } = render(NewRoutineModal, {
      props: { open: false, onclose: () => {} },
    });
    expect(queryByText('새 루틴')).toBeNull();
  });
});
