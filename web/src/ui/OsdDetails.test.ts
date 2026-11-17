import { describe, expect, it } from 'vitest';
import { whepStats } from '@/core/whep';
import { FlyState } from '@/ui/FlyState';
import { NOT_FULLY_READY, RC_NO_SIGNAL, bitrateText, ekfRatiosOf, ekfWorst, osdCellText, osdLines, osdRows, percentText, playingStats, redIndicators, withRedIndicators } from '@/ui/OsdDetails';

const quiet = { hdop: NaN, ekfRatios: [], vibration: null, video: null };
const counters = { bytes: 0, lost: 0, received: 0, dropped: 0, at: 0 };
const healthyVideo = { fps: 30, kbps: 2200, jitterMs: 10, lossPercent: 0.4, lostPerSecond: 0, skippedPerSecond: 0, counters };

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
      ['ic_videocam', 'Bitrate 2.2 Mb/s · Frame rate 30 fps · Jitter 10 ms · Packet loss 0.4% · Skipped frames 0/s'],
    ]);
    const lines = osdLines({ ...everything, ekfRatios: [0.62], video: { ...healthyVideo, lossPercent: 12, lostPerSecond: 0, skippedPerSecond: 2 } });
    expect(lines.map(osdCellText)).toEqual(['EKF variance 0.62 · Vibration 3.2 · HDOP 0.5', 'Packet loss 12% · Skipped frames 2/s · Bitrate 2.2 Mb/s · Frame rate 30 fps · Jitter 10 ms']);
    expect(osdRows(lines[1]!).map((it) => it.severity)).toEqual([0, 0, 0, 2, 1]);
  });

  it('a camera that reports lost packets but not how many arrived shows the loss as a rate', () => {
    const rtsp = osdLines({ ...everything, video: { ...healthyVideo, lossPercent: null, lostPerSecond: 3.4 } });
    expect(osdCellText(rtsp[1]!)).toBe('Bitrate 2.2 Mb/s · Frame rate 30 fps · Jitter 10 ms · Packets lost 3/s · Skipped frames 0/s');
  });

  it('vibration says its unit, the level it is judged against and what to do about it', () => {
    const shaking = { ...everything, vibration: { units: 'm/s²', axes: [{ axis: 'Z', value: 60, fraction: 0, severity: 'danger' }], clipCounts: [], warningLevel: 30, dangerLevel: 60, scaleMaximum: 90 } };
    const health = osdLines(shaking)[0]!;
    expect(osdCellText(health)).toBe('Vibration 60.0m/s² · EKF variance 0.32 · HDOP 0.5');
    expect(osdRows(health)[1]).toMatchObject({ label: 'Vibration', value: '60.0m/s²', severity: 2 });
    expect(osdRows(health)[1]?.hint).toMatch(/^At or over the 60 m\/s² danger level\. /);
  });

  it('a red reading or a silent RC turns Ready to fly into Not fully ready, and says why', () => {
    const ready = FlyState({ connected: true, armed: false, contactLost: false, state: 'disarmed', stateText: 'Ready to fly', staleNotice: '', mode: 'Stabilize', rcSupported: true, rcRequired: true, rcSignalText: 'No signal', rcSignal: 0, rcOverride: false, telemetry: null, summaryDetail: 'All checks passed.' });
    const shaking = osdLines({ ...everything, vibration: { units: 'm/s²', axes: [{ axis: 'Z', value: 60, fraction: 0, severity: 'danger' }], clipCounts: [], warningLevel: 30, dangerLevel: 60, scaleMaximum: 90 } });
    const red = redIndicators(shaking, ready);
    expect(red).toEqual(['Vibration 60.0m/s²', RC_NO_SIGNAL]);
    expect(withRedIndicators(ready, red)).toMatchObject({ nominal: false, stateText: NOT_FULLY_READY, summaryDetail: 'Vibration 60.0m/s². RC has no signal. Tap a red reading to see how to fix it.' });
    expect(withRedIndicators({ ...ready, armed: true }, red)?.stateText, 'in the air the chip keeps saying what the aircraft is doing').toBe('Ready to fly');
    expect(redIndicators(osdLines(everything), { ...ready, rcRequired: false })).toEqual([]);
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
    expect([bitrateText(now.kbps), bitrateText(800)]).toEqual(['2.2 Mb/s', '800 kb/s']);
    expect([percentText(0.44), percentText(12.4)]).toEqual(['0.4%', '12%']);
  });
});
