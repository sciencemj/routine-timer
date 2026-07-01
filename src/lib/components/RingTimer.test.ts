import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import RingTimer from './RingTimer.svelte';

describe('RingTimer', () => {
  it('renders an svg element', () => {
    const { container } = render(RingTimer, { props: { progress: 0.5, label: '12:30' } });
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('shows the label text inside svg', () => {
    const { getByText } = render(RingTimer, { props: { progress: 0.25, label: '05:00' } });
    expect(getByText('05:00')).toBeTruthy();
  });

  it('renders with default props without error', () => {
    const { container } = render(RingTimer);
    expect(container.querySelector('svg')).not.toBeNull();
  });
});
