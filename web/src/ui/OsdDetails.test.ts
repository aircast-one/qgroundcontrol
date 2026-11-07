import { describe, expect, it } from 'vitest';
import { whepStats } from '@/core/whep';
import { bitrateText, ekfWorst, osdDetails } from '@/ui/OsdDetails';

const quiet = { battery: null, gps: null, hdop: NaN, ekfRatios: [], vibration: null, contactLost: false, lossPercent: NaN, video: null, radio: null };

describe('OsdDetailsTest', () => {
  it('shows every reading the aircraft and the video report', () => {
    const details = osdDetails({
      battery: { packs: [{ voltageText: '43.58V', currentText: '1.3A' }] },
      gps: { available: true, lockText: '3D Lock', satellites: 27 },
      hdop: 0.54,
      ekfRatios: [0.1, NaN, 0.62],
      vibration: { axes: [{ axis: 'X', value: 3.2, fraction: 0, severity: 'normal' }], clipping: [], warningLevel: 30, dangerLevel: 60, scaleMaximum: 90 } as never,
      contactLost: false,
      lossPercent: 0.4,
      video: { fps: 30, kbps: 2200, jitterMs: 10, lost: 0, dropped: 2, bytes: 0, at: 0 },
      radio: { text: 'Radio · live', state: 'live' } as never,
    });
    expect(details.map((it) => `${it.label} ${it.value}`)).toEqual([
      'BAT 43.58V 1.3A',
      'GPS 3D Lock',
      'SAT 27',
      'HDOP 0.5',
      'EKF 0.62',
      'VIBE 3.2',
      'TLM LIVE',
      'LOSS 0%',
      'VIDEO 30fps 2.2Mb/s',
      'JIT 10ms',
      'LOST 0',
      'DROP 2',
      'RADIO LIVE',
    ]);
    expect(details.find((it) => it.label === 'EKF')?.tone).toBe('warning');
    expect(details.find((it) => it.label === 'DROP')?.tone).toBe('warning');
  });

  it('a lost link and a missing video stand out, and silent sensors stay hidden', () => {
    const details = osdDetails({ ...quiet, contactLost: true });
    expect(details).toEqual([
      { label: 'TLM', value: 'LOST', tone: 'error' },
      { label: 'VIDEO', value: 'OFF', tone: 'warning' },
    ]);
  });

  it('the EKF reads its worst reported variance', () => {
    expect(ekfWorst([NaN, NaN])).toBeNull();
    expect(ekfWorst([0.2, 0.9, NaN])).toBe(0.9);
  });

  it('bitrate and stream stats come from successive WebRTC reports', () => {
    const first = whepStats({ timestamp: 1000, framesPerSecond: 29.6, bytesReceived: 0, jitter: 0.012, packetsLost: 1, framesDropped: 0 }, null);
    const second = whepStats({ timestamp: 2000, framesPerSecond: 30, bytesReceived: 275_000, jitter: 0.01, packetsLost: 1, framesDropped: 0 }, first);
    expect([first.fps, first.kbps, first.jitterMs]).toEqual([30, 0, 12]);
    expect(second.kbps).toBe(2200);
    expect(bitrateText(second.kbps)).toBe('2.2Mb/s');
    expect(bitrateText(800)).toBe('800kb/s');
  });
});
