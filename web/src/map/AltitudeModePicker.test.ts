import { describe, expect, it } from 'vitest';
import { GLOBAL_FRAME_MIXED, GLOBAL_FRAME_RELATIVE, itemReferenceSelectable, itemReferenceShown } from './AltitudeModePicker';

describe('ScanPatternsTest', () => {
  it("an item's altitude reference follows the plan-wide frame, like SimpleItemEditor", () => {
    const frames = [GLOBAL_FRAME_MIXED, GLOBAL_FRAME_RELATIVE, 2, null];
    expect(frames.map(itemReferenceShown)).toEqual([true, false, true, true]);
    expect(frames.map(itemReferenceSelectable)).toEqual([true, false, false, true]);
  });
});
