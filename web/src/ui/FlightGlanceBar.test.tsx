// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { FlightGlanceBar } from './ArmedFlightBar';

afterEach(cleanup);

const glance = { mode: 'Auto', altitude: '61.0 m', battery: '84%', batteryLow: false, link: 'OK', linkLost: false, dropped: false };

describe('FlightGlanceBarTest', () => {
  it('ticking height and battery are not a live region, so a screen reader is not read every tick', () => {
    const { container } = render(<FlightGlanceBar glance={glance} onFly={() => undefined} />);
    expect(container.querySelector('[role="status"], [aria-live]')).toBeNull();
    expect(screen.getByRole('group', { name: 'Flight status' })).not.toBeNull();
  });

  it('a lost link still alerts', () => {
    render(<FlightGlanceBar glance={{ ...glance, dropped: true, mode: 'Link lost · Copter' }} onFly={() => undefined} />);
    expect(screen.getByRole('alert')).not.toBeNull();
  });

  it('Back to Fly sits outside the scrolling readings and never wraps', () => {
    render(<FlightGlanceBar glance={glance} onFly={() => undefined} />);
    const button = screen.getByRole('button', { name: 'Back to Fly' });
    expect(button.className).toContain('whitespace-nowrap');
    expect(button.className).toContain('shrink-0');
    expect(button.parentElement?.getAttribute('role')).toBe('group');
  });
});
