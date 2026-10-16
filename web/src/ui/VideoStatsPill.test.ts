import { describe, expect, it } from 'vitest';
import { videoStatsText } from '@/ui/VideoStatsPill';

describe('VideoStatsPillTest', () => {
  it("the pill shows the core's stats line", () => {
    expect(videoStatsText({ text: '30 fps · 720p' })).toBe('30 fps · 720p');
    expect(videoStatsText(null)).toBe('');
  });
});
