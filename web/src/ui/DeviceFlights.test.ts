import { describe, expect, it } from 'vitest';
import { DeviceLevel } from '@/ui/DeviceStatus';
import {
  clockAt,
  composeZoom,
  flightLink,
  formatValue,
  gridLines,
  hasLines,
  laneKeys,
  markerLines,
  parseFlights,
  parseRenderJob,
  readoutAt,
  renderPercent,
  renderRunning,
  segments,
  spanLabel,
  ticks,
  toggled,
  zoomSlice,
} from '@/ui/DeviceFlights';

const marker = (offset: number, label: string) => ({ offset, label, level: DeviceLevel.Neutral });

describe('DeviceFlightsTest', () => {
  it('reads the flight log the device serves', () => {
    const view = parseFlights({
      flights: [
        {
          id: 'f1',
          title: 'Tue 22 Sep, 14:00',
          summary: 'Lasted 22m10s',
          badge: { level: 3, text: '3 events' },
          sections: [
            {
              title: 'Sortie 1',
              span: '14:02:10 – 14:12:40 · 10m30s',
              videos: [{ camera: 'cam2', label: 'Video · cam2', url: '/api/recordings/cam2/video?from=a&to=b', share: { label: 'Share version · cam2', confirm: 'Send about 1.2 MB?', camera: 'cam2', from: 'a', to: 'b' } }],
              lines: [{ index: 1, clock: '14:02:14', text: 'cam stopped sending video', level: 2, alert: true, repeats: 'repeated ×4 until 14:02:44' }],
            },
          ],
          trace: {
            step: 30,
            startUnix: 1790085600,
            tzOffset: 10800,
            lanes: [{ key: 'signal', label: 'Modem signal', chip: 'Signal', values: [80, 41], notes: ['4G', '3G'], unit: 'percent', max: 100, default: true, peak: '41%' }],
            markers: [{ offset: 60, label: '14:02:05 modem', level: 2 }],
            spans: [{ label: 'Sortie 1', from: 0.1, to: 0.9 }],
          },
          csv: '/api/flights/f1/samples.csv',
          json: '/api/flights/f1',
        },
        { id: 'f2', title: 'Wed', badge: { level: 0, text: 'in progress' }, open: true, sections: [], dropped: '12 more events happened' },
      ],
    });
    const [traced, live] = view?.flights ?? [];
    expect(traced?.sections[0]?.lines[0]).toEqual({ index: 1, clock: '14:02:14', text: 'cam stopped sending video', level: DeviceLevel.Blocked, alert: true, repeats: 'repeated ×4 until 14:02:44', heldBack: false });
    expect(traced?.sections[0]?.videos[0]?.share?.camera).toBe('cam2');
    expect(traced?.trace?.lanes[0]).toMatchObject({ key: 'signal', values: [80, 41], notes: ['4G', '3G'], max: 100, default: true });
    expect(traced?.trace?.markers[0]?.level).toBe(DeviceLevel.Blocked);
    expect(traced?.trace?.spans[0]).toEqual({ label: 'Sortie 1', from: 0.1, to: 0.9 });
    expect(traced !== undefined && hasLines(traced)).toBe(true);
    expect(live?.trace).toBeNull();
    expect(live?.open).toBe(true);
    expect(live !== undefined && hasLines(live)).toBe(false);
    expect(parseFlights({})).toBeNull();
    expect(parseFlights({ flights: [] })?.flights).toEqual([]);
  });

  it('a copied link opens the fly app straight on that flight', () => {
    expect(flightLink('http://falcon.local/fly/', '20260922-140000')).toBe('http://falcon.local/fly/#device=%2Fflights%3Fflight%3D20260922-140000');
  });

  it('a flight opens with its own defaults until someone chooses', () => {
    expect(laneKeys(null, ['signal', 'bitrate'])).toEqual(['signal', 'bitrate']);
    expect(laneKeys(['cpu'], ['signal', 'bitrate'])).toEqual(['cpu']);
  });

  it('choosing nothing stays chosen', () => {
    expect(laneKeys([], ['signal'])).toEqual([]);
  });

  it('a chip adds the lane it names and takes it away again', () => {
    expect(toggled(['signal'], 'cpu')).toEqual(['signal', 'cpu']);
    expect(toggled(['signal', 'cpu'], 'signal')).toEqual(['cpu']);
  });

  it('the line breaks where the value is unknown', () => {
    expect(segments([100, 50, -1, 0, 100], 100)).toEqual(['0.0,1.0 25.0,10.0', '75.0,19.0 100.0,1.0']);
  });

  it('without a ceiling the line scales to its own peak', () => {
    expect(segments([0, 250000, 500000], 0)).toEqual(['0.0,19.0 50.0,10.0 100.0,1.0']);
  });

  it('a single point draws nothing', () => {
    expect(segments([42], 100)).toEqual([]);
    expect(segments([], 100)).toEqual([]);
  });

  it('a lane gets evenly spaced clock ticks from first to last sample', () => {
    const t = ticks(1_700_000_000, 30, 13, (unix) => String(unix));
    expect(t.map((it) => it.x)).toEqual([0, 25, 50, 75, 100]);
    expect(t[0]?.label).toBe('1700000000');
    expect(t[4]?.label).toBe('1700000360');
    expect(ticks(1, 30, 1, String)).toEqual([]);
  });

  it('markers land where their moment falls on the series, or not at all', () => {
    expect(markerLines([marker(0, 'a'), marker(180, 'b'), marker(361, 'late'), marker(-1, 'early')], 30, 13)).toEqual([
      { x: 0, label: 'a', level: DeviceLevel.Neutral, index: 0 },
      { x: 50, label: 'b', level: DeviceLevel.Neutral, index: 1 },
    ]);
  });

  it('a zoomed lane keeps each marker on its moment and its index on its line', () => {
    expect(markerLines([marker(30, 'before'), marker(90, 'inside')], 30, 3, 60)).toEqual([{ x: 50, label: 'inside', level: DeviceLevel.Neutral, index: 1 }]);
  });

  it('the caption names the span in minutes or hours', () => {
    expect(spanLabel(360)).toBe('6m');
    expect(spanLabel(4500)).toBe('1h 15m');
  });

  it("the axis reads in the device's clock, not the browser's", () => {
    expect(clockAt(1_700_000_000, 0, false)).toBe('22:13');
    expect(clockAt(1_700_000_000, 3 * 3600, true)).toBe('01:13:20');
  });

  it("gridlines sit at quarters of the ceiling with the unit's label", () => {
    expect(gridLines(100, 'percent').map((it) => it.label)).toEqual(['25%', '50%', '75%']);
    expect(gridLines(250000, 'rate').map((it) => it.label)).toEqual(['500 kbps', '1.0 Mbps', '1.5 Mbps']);
    expect(gridLines(0, 'percent')).toEqual([]);
  });

  it('a hover reads the nearest sample with its note', () => {
    expect(readoutAt([80, 41, 18], ['4G (LTE)', '3G', '3G'], 0.9, 'percent')).toEqual({ index: 2, x: 100, text: '18% · 3G' });
    expect(readoutAt([80, -1], [], 1, 'percent')?.text).toBe('no reading');
    expect(readoutAt([], [], 0.5, 'rate')).toBeNull();
  });

  it('counts read as bare numbers', () => {
    expect(formatValue(512.4, 'count')).toBe('512');
  });

  it('distances read in metres until a kilometre', () => {
    expect(formatValue(640, 'meters')).toBe('640 m');
    expect(formatValue(1250, 'meters')).toBe('1.3 km');
    expect(gridLines(2000, 'meters').map((it) => it.label)).toEqual(['500 m', '1.0 km', '1.5 km']);
  });

  it('a zoom window picks a run of samples and never fewer than two', () => {
    expect(zoomSlice(14, null)).toEqual({ from: 0, to: 13 });
    expect(zoomSlice(14, [0.25, 0.5])).toEqual({ from: 3, to: 7 });
    expect(zoomSlice(14, [0.99, 1])).toEqual({ from: 12, to: 13 });
    expect(zoomSlice(14, [0.3, 0.31])).toEqual({ from: 4, to: 5 });
  });

  it('a drag inside a zoom narrows the zoom it is in', () => {
    expect(composeZoom(null, 0.6, 0.2)).toEqual([0.2, 0.6]);
    expect(composeZoom([0.5, 1], 0, 0.5)).toEqual([0.5, 0.75]);
  });

  it("each lane's unit is spelled the way that lane is read", () => {
    expect(formatValue(0.72, 'number')).toBe('0.7');
    expect(formatValue(2048, 'mb')).toBe('2.0 GB');
    expect(formatValue(512, 'mb')).toBe('512 MB');
    expect(formatValue(45, 'seconds')).toBe('45 s');
    expect(formatValue(180, 'seconds')).toBe('3 m');
    expect(formatValue(73, 'percent')).toBe('73%');
    expect(formatValue(4, 'count')).toBe('4');
  });

  it('a render reports how far it got and why it stopped', () => {
    const uploading = parseRenderJob({ id: 'r1', state: 'uploading', sent: 300, total: 1200 });
    expect(renderRunning(uploading)).toBe(true);
    expect(renderPercent(uploading)).toBe(25);
    expect(renderPercent(parseRenderJob({ state: 'uploading' }))).toBe(0);
    expect(renderRunning(parseRenderJob({ state: 'done', url: 'https://r2/x.mp4' }))).toBe(false);
    expect(parseRenderJob({ state: 'failed' }).error).toBe('The cloud could not render this video.');
    expect(parseRenderJob({ state: 'failed', error: 'the cloud could not render this video' }).error).toBe('the cloud could not render this video');
  });
});
