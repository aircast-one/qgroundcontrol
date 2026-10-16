import { describe, expect, it } from 'vitest';
import { trafficLabel, trafficMarks } from './TrafficLayer';

describe('TrafficLayerTest', () => {
  it('contacts become marks labelled as the map item labels them', () => {
    const marks = trafficMarks({
      units: { altitude: 'ft' },
      contacts: [
        { latitude: 47.4, longitude: 8.5, headingDegrees: 90, alert: true, altitude: 3280.8, callsign: 'UAL123' },
        { latitude: null, longitude: 8.5, alert: false, altitude: 100, callsign: 'X' },
      ],
    });
    expect(marks.length).toBe(1);
    expect(marks[0]?.label).toBe('3281 ft\nUAL123');
    expect(marks[0]?.alert).toBe(true);
    expect(trafficLabel(null, 'ft', 'UAL123')).toBe('');
  });
});
