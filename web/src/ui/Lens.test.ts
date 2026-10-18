import { describe, expect, it } from 'vitest';
import { lensReading, measuredText } from '@/ui/Lens';

describe('lens', () => {
  it('reads what the core says about the lens and how it was learned', () => {
    const reading = lensReading({ kind: 'object', class: 'Lens', summary: '62° × 37° · measured', approximate: false, measuring: false, progress: '', error: null, canMeasure: true, reason: null, samples: [{ zoom: 50, hFov: 12, vFov: 7 }, { zoom: 0, hFov: 62, vFov: 37 }] });
    expect(reading).toMatchObject({ summary: '62° × 37° · measured', canMeasure: true, error: null, reason: null, measuredZooms: [50, 0] });
    expect(measuredText(reading?.measuredZooms ?? [])).toBe('Measured at 0%, 50% zoom');
    expect(lensReading({ kind: 'object', class: 'Camera' })).toBeNull();
    expect(measuredText([])).toBeNull();
  });
});
