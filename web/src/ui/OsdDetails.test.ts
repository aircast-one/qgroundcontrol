import { describe, expect, it } from 'vitest';
import { whepStats } from '@/core/whep';
import { type OsdLine, bitrateText, ekfWorst, osdLines, percentText, shownDetails } from '@/ui/OsdDetails';

const quiet = { hdop: NaN, ekfRatios: [], vibration: null, contactLost: false, lossPercent: NaN, video: null, radio: null };
const counters = { bytes: 0, lost: 0, received: 0, dropped: 0, at: 0 };
const healthyVideo = { fps: 30, kbps: 2200, jitterMs: 10, lossPercent: 0.4, skippedPerSecond: 0, counters };

const shown = (lines: readonly OsdLine[], opened = false) => lines.map((line) => [line.icon, shownDetails(line, opened).map((it) => `${it.label} ${it.value}`).join(' · ')]);

const everything = {
  hdop: 0.54,
  ekfRatios: [0.1, NaN, 0.32],
  vibration: { axes: [{ axis: 'X', value: 3.2, fraction: 0, severity: 'normal' }], clipping: [], warningLevel: 30, dangerLevel: 60, scaleMaximum: 90 } as never,
  contactLost: false,
  lossPercent: 0.4,
  video: healthyVideo,
  radio: { text: 'Radio · live', state: 'live' as const },
};

describe('OsdDetailsTest', () => {
  it('a healthy group shows its icon and one key reading', () => {
    expect(shown(osdLines(everything))).toEqual([
      ['ic_explore', 'EKF 0.32'],
      ['ic_sensors', 'TLM LIVE'],
      ['ic_videocam', 'VIDEO 30fps'],
      ['ic_gamepad', 'RADIO LIVE'],
    ]);
  });

  it('an opened group shows every reading it has', () => {
    expect(shown(osdLines(everything), true)).toEqual([
      ['ic_explore', 'EKF 0.32 · VIB 3.2 · HDOP 0.5'],
      ['ic_sensors', 'TLM LIVE · TLM LOSS 0.4%'],
      ['ic_videocam', 'VIDEO 30fps · BR 2.2Mb/s · JIT 10ms · PKT LOSS 0.4% · SKIP 0/s'],
      ['ic_gamepad', 'RADIO LIVE'],
    ]);
  });

  it('a group with something wrong opens by itself and colours the culprit', () => {
    const lines = osdLines({ ...everything, ekfRatios: [0.62], video: { ...healthyVideo, lossPercent: 12, skippedPerSecond: 2 } });
    expect(shown(lines)).toEqual([
      ['ic_explore', 'EKF 0.62 · VIB 3.2 · HDOP 0.5'],
      ['ic_sensors', 'TLM LIVE'],
      ['ic_videocam', 'VIDEO 30fps · BR 2.2Mb/s · JIT 10ms · PKT LOSS 12% · SKIP 2/s'],
      ['ic_gamepad', 'RADIO LIVE'],
    ]);
    const video = lines[2]?.details ?? [];
    expect(video.map((it) => it.tone)).toEqual([null, null, null, 'error', 'warning']);
    expect(lines[0]?.details[0]?.tone).toBe('warning');
  });

  it('a lost link and a missing video stand out, and silent sensors stay hidden', () => {
    expect(osdLines({ ...quiet, contactLost: true })).toEqual([
      { id: 'link', icon: 'ic_sensors', details: [{ label: 'TLM', value: 'LOST', tone: 'error' }] },
      { id: 'video', icon: 'ic_videocam_off', details: [{ label: 'VIDEO', value: 'OFF', tone: 'warning' }] },
    ]);
  });

  it('the EKF reads its worst reported variance', () => {
    expect(ekfWorst([NaN, NaN])).toBeNull();
    expect(ekfWorst([0.2, 0.9, NaN])).toBe(0.9);
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
