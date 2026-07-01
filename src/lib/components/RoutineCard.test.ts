import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import RoutineCard from './RoutineCard.svelte';
import type { Routine } from '../types';

const baseRoutine: Routine = {
  id: 1, name: '딥워크', icon: '🎯', color: null,
  target_seconds: 3600, pomodoro_enabled: false,
  focus_minutes: 25, break_minutes: 5,
  sort_order: 1, archived: false, created_at: '2024-01-01T00:00:00Z',
};

describe('RoutineCard', () => {
  it('renders the routine name', () => {
    const { getByText } = render(RoutineCard, {
      props: { routine: baseRoutine, todaySecs: 0 },
    });
    expect(getByText('딥워크')).toBeTruthy();
  });

  it('shows 미시작 when todaySecs is 0 and not active', () => {
    const { getByText } = render(RoutineCard, {
      props: { routine: baseRoutine, todaySecs: 0, active: false },
    });
    expect(getByText('미시작')).toBeTruthy();
  });

  it('shows 일부 진행 when todaySecs > 0 and not complete', () => {
    const { getByText } = render(RoutineCard, {
      props: { routine: baseRoutine, todaySecs: 900, active: false },
    });
    expect(getByText('일부 진행')).toBeTruthy();
  });

  it('shows 완료 when todaySecs >= target_seconds', () => {
    const { getByText } = render(RoutineCard, {
      props: { routine: baseRoutine, todaySecs: 3600, active: false },
    });
    expect(getByText('완료')).toBeTruthy();
  });

  it('shows 진행 중 when active regardless of todaySecs', () => {
    const { getByText } = render(RoutineCard, {
      props: { routine: baseRoutine, todaySecs: 0, active: true },
    });
    expect(getByText('진행 중')).toBeTruthy();
  });

  it('shows 진행 중 even when todaySecs >= target when active', () => {
    const { getByText } = render(RoutineCard, {
      props: { routine: baseRoutine, todaySecs: 3600, active: true },
    });
    expect(getByText('진행 중')).toBeTruthy();
  });
});
