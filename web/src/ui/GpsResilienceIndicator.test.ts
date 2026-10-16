import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { gpsResilience } from './GpsResilienceIndicator';

describe('GpsResilienceIndicatorTest', () => {
  it('marks and sections come from the core', () => {
    const shown = gpsResilience(
      parseJson(
        '{"shown":true,"interference":{"shown":true,"colour":"error"},"authentication":{"shown":false,"colour":"neutral"},' +
          '"sections":[{"title":"GPS Resilience Status","rows":[{"label":"GPS Jamming","text":"Not jammed"}]}]}',
      ),
    )!;
    expect(shown.interference.shown).toBe(true);
    expect(shown.interference.colour).toBe('error');
    expect(shown.sections[0]!.rows[0]).toEqual(['GPS Jamming', 'Not jammed']);
    expect(gpsResilience(parseJson('{"shown":false}'))).toBeNull();
  });
});
