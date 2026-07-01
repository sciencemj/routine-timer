import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { resetTimer } from '$lib/timer.svelte';
import Report from './Report.svelte';
import type { ReportData } from '$lib/types';

const reportFixture: ReportData = {
  heatmap: [
    { date: '2026-06-28', secs: 0, level: 0 },
    { date: '2026-06-29', secs: 1200, level: 1 },
    { date: '2026-06-30', secs: 5400, level: 3 },
    { date: '2026-07-01', secs: 9000, level: 4 },
  ],
  this_week_secs: 12000,
  last_week_secs: 9000,
  daily_avg_secs: 3600,
  month_active_days: 12,
  streak: 5,
  best_streak: 10,
  last7: [0, 1200, 3600, 0, 5400, 7200, 9000],
};

describe('Report', () => {
  beforeEach(() => {
    resetTauri();
    resetTimer();
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'stats_report' ? Promise.resolve(reportFixture) : Promise.resolve()
    );
  });

  it('renders 집중 기록 heading', async () => {
    const { findByText } = render(Report);
    expect(await findByText('집중 기록')).toBeInTheDocument();
  });

  it('renders the 이번 주 집중 KPI value', async () => {
    const { findByText } = render(Report);
    expect(await findByText('3시간 20분')).toBeInTheDocument();
  });
});
