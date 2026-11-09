import { describe, expect, it } from 'vitest';
import { whepStats } from '@/core/whep';
import { bitrateText, ekfRatiosOf, ekfWorst, osdCellText, osdLines, osdRows, percentText, playingStats } from '@/ui/OsdDetails';

const quiet = { hdop: NaN, ekfRatios: [], vibration: null, video: null };
const counters = { bytes: 0, lost: 0, received: 0, dropped: 0, at: 0 };
const healthyVideo = { fps: 30, kbps: 2200, jitterMs: 10, lossPercent: 0.4, skippedPerSecond: 0, counters };

const everything = {
  hdop: 0.54,
  ekfRatios: [0.1, NaN, 0.32],
  vibration: { axes: [{ axis: 'X', value: 3.2, fraction: 0, severity: 'normal' }], clipping: [], warningLevel: 30, dangerLevel: 60, scaleMaximum: 90 } as never,
  video: healthyVideo,
};

describe('OsdDetailsTest', () => {
  it('each status bar cell reads its worst reading first, then the rest in order', () => {
    expect(osdLines(everything).map((line) => [line.icon, osdCellText(line)])).toEqual([
      ['ic_explore', 'EKF variance 0.32 · Vibration 3.2 · HDOP 0.5'],
      ['ic_videocam', 'Bitrate 2.2Mb/s · Frame rate 30fps · Jitter 10ms · Packet loss 0.4% · Skipped frames 0/s'],
    ]);
    const lines = osdLines({ ...everything, ekfRatios: [0.62], video: { ...healthyVideo, lossPercent: 12, skippedPerSecond: 2 } });
    expect(lines.map(osdCellText)).toEqual(['EKF variance 0.62 · Vibration 3.2 · HDOP 0.5', 'Packet loss 12% · Skipped frames 2/s · Bitrate 2.2Mb/s · Frame rate 30fps · Jitter 10ms']);
    expect(osdRows(lines[1]!).map((it) => it.severity)).toEqual([0, 0, 0, 2, 1]);
  });

  it('no video and no sensor readings leave no cells, so nothing says VIDEO OFF', () => {
    expect(osdLines(quiet)).toEqual([]);
  });

  it('the strip reads whichever main video surface is still playing', () => {
    const stats = { ...healthyVideo, fps: 25 };
    expect(playingStats(new Map())).toBeNull();
    expect(playingStats(new Map([[Symbol('gone'), stats]]))).toBe(stats);
  });

  it('the EKF reads its worst reported variance from the estimator facts', () => {
    const facts = [
      { name: 'velRatio', value: 0.2 },
      { name: 'magRatio', value: 0.9 },
      { name: 'haglRatio', value: null },
      { name: 'goodAttitudeEsimate', value: 1 },
    ] as never;
    expect(ekfRatiosOf(facts)).toEqual([0.2, 0.9, NaN]);
    expect(ekfWorst(ekfRatiosOf(facts))).toBe(0.9);
    expect(ekfWorst([NaN, NaN])).toBeNull();
  });

  it('loss and skipped frames are rates over the recent window, not totals since connecting', () => {
    const start = whepStats({ timestamp: 1000, framesPerSecond: 29.6, bytesReceived: 1_000_000, jitter: 0.012, packetsLost: 700, packetsReceived: 90_000, framesDropped: 14 }, null);
    expect([start.fps, start.kbps, start.jitterMs, start.lossPercent, start.skippedPerSecond]).toEqual([30, 0, 12, 0, 0]);
    const now = whepStats({ timestamp: 3000, framesPerSecond: 30, bytesReceived: 1_550_000, jitter: 0.01, packetsLost: 710, packetsReceived: 90_490, framesDropped: 18 }, start.counters);
    expect(now.kbps).toBe(2200);
    expect(now.lossPercent).toBe(2);
    expect(now.skippedPerSecond).toBe(2);
    expect([bitrateText(now.kbps), bitrateText(800)]).toEqual(['2.2Mb/s', '800kb/s']);
    expect([percentText(0.44), percentText(12.4)]).toEqual(['0.4%', '12%']);
  });
});
