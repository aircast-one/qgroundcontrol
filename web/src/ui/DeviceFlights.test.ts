import { describe, expect, it } from 'vitest';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { composeZoom, flightLink, hasLines, laneKeys, parseFlightChart, parseFlights, parseRenderJob, renderPercent, renderRunning, sampleAt, toggled } from '@/ui/DeviceFlights';

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
    expect(traced?.trace).toMatchObject({ step: 30, startUnix: 1790085600, spans: [{ label: 'Sortie 1', from: 0.1, to: 0.9 }] });
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

  it("draws the chart the core computed in the core's plot units", () => {
    const chart = parseFlightChart({
      lanes: [
        {
          key: 'signal',
          label: 'Modem signal',
          chip: 'Signal',
          peak: '41%',
          default: true,
          lines: [[[0, 5], [25, 50.5]], [[75, 95], [100, 5]]],
          grid: [{ y: 72.5, label: '25%' }],
          markers: [{ x: 50, label: '14:02:05 modem', index: 1 }],
          readings: ['14:00:00 · 80% · 4G', '14:00:30 · no reading'],
          ticks: [{ x: 0, label: '14:00' }],
          span: '6m of flight',
        },
      ],
      spans: [{ label: 'Sortie 1', from: 0.1, to: 0.9 }],
    });
    const lane = chart?.lanes[0];
    expect(lane?.lines).toEqual(['0,5 25,50.5', '75,95 100,5']);
    expect(lane?.markers[0]).toEqual({ x: 50, label: '14:02:05 modem', level: DeviceLevel.Neutral, index: 1 });
    expect(lane?.grid[0]).toEqual({ y: 72.5, label: '25%' });
    expect(lane?.readings[1]).toBe('14:00:30 · no reading');
    expect(lane?.default).toBe(true);
    expect(chart?.spans[0]).toEqual({ label: 'Sortie 1', from: 0.1, to: 0.9 });
    expect(parseFlightChart(null)).toBeNull();
  });

  it('a cursor snaps to the nearest sample of the lane it is over', () => {
    expect(sampleAt(3, 0.9)).toEqual({ index: 2, x: 100 });
    expect(sampleAt(5, 0.3)).toEqual({ index: 1, x: 25 });
    expect(sampleAt(1, 0.7)).toEqual({ index: 0, x: 0 });
    expect(sampleAt(0, 0.5)).toBeNull();
  });

  it('a drag inside a zoom narrows the zoom it is in', () => {
    expect(composeZoom(null, 0.6, 0.2)).toEqual([0.2, 0.6]);
    expect(composeZoom([0.5, 1], 0, 0.5)).toEqual([0.5, 0.75]);
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
