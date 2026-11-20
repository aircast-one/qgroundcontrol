import { describe, expect, it } from 'vitest';
import { flightTitle, recordedFlights } from '@/ui/RecordedFlights';

describe('RecordedFlightsTest', () => {
  it('a saved telemetry log reads as the flight it holds', () => {
    expect(flightTitle('2026-11-20 06-30-07.tlog')).toBe('Flight on 2026-11-20 at 06:30');
    expect(flightTitle('2026-11-20 06-30-07.1.tlog'), 'a second log in the same second still names its flight').toBe('Flight on 2026-11-20 at 06:30');
    expect(flightTitle('imported.tlog')).toBe('imported.tlog');
  });

  it('the core list keeps each log with the path Review plays', () => {
    expect(recordedFlights([{ name: '2026-11-20 06-30-07.tlog', path: '/Aircast/Telemetry/2026-11-20 06-30-07.tlog', size: 10 }, 'junk'])).toEqual([
      { name: '2026-11-20 06-30-07.tlog', path: '/Aircast/Telemetry/2026-11-20 06-30-07.tlog' },
    ]);
    expect(recordedFlights(null)).toEqual([]);
  });
});
