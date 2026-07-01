import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import Report from './Report.svelte';

describe('Report', () => {
  it('renders 집중 기록 heading', () => {
    const { getByText } = render(Report);
    expect(getByText('집중 기록')).toBeInTheDocument();
  });

  it('renders placeholder copy', () => {
    const { getByText } = render(Report);
    expect(getByText('리포트는 곧 추가됩니다')).toBeInTheDocument();
  });
});
