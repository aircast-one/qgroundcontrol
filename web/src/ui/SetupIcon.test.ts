import { describe, expect, it } from 'vitest';
import { setupIcon, setupNote } from '@/ui/SetupScreen';

describe('SetupIconTest', () => {
  it('aKnownPageKeepsItsIcon', () => {
    expect(setupIcon('radio', 'APMRadioComponent')).toBe('ic_gamepad');
  });

  it('anUnknownPageFallsBackToItsUntranslatedClass', () => {
    expect(setupIcon(null, 'APMFailsafesComponent')).toBe('ic_warning');
    expect(setupIcon(null, 'APMAdvancedTuningCopterComponent')).toBe('ic_tune');
    expect(setupIcon(null, 'SomethingNewComponent')).toBe('ic_build');
  });

  it('theNoteFollowsTheMostSpecificClassToken', () => {
    expect(setupNote('APMAdvancedTuningCopterComponent')).toBe('Every rate and filter, per axis');
    expect(setupNote('APMTuningComponent')).toBe('How it responds to the sticks');
    expect(setupNote('APMFlightSafetyComponent')).toBe('Return altitude, landing speed and geofence');
    expect(setupNote('SomethingNewComponent')).toBe('');
  });
});
