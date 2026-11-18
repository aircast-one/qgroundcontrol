// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { PlanItemList } from './PlanItemList';
import { ItemRow } from './PlanItems';

afterEach(cleanup);

describe('PlanItemList', () => {
  it('a long sequence range widens its marker instead of being cut to fit a circle', () => {
    const rows = [ItemRow({ index: 4, number: '102–103', name: 'Waypoint', detail: '120 m · 6.0 m/s', colour: '#FFB300', placed: true })];
    render(<PlanItemList rows={rows} summary="" selected={null} headingClassName="" movesOf={() => ({ earlier: false, later: false })} onPick={() => undefined} onMove={() => undefined} onDelete={null} />);
    const marker = screen.getByText('102–103').parentElement!;
    expect(marker.style.width, 'a fixed 40 px circle clipped 102–103 to 102–1').toBe('');
    expect(marker.style.minWidth).toBe('40px');
  });
});
