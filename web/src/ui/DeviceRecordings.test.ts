import { describe, expect, it } from 'vitest';
import { type RecordingSegment, lowersLimit, parseRecordings, recordPath, segmentDetail, segmentPath } from '@/ui/DeviceRecordings';

const segment = (fields: Partial<RecordingSegment>): RecordingSegment => ({ at: '2026-10-18T10:00:00+04:00', started: '18 Oct 10:00:00', duration: '10 min', size: '1.0 MB', url: '/v', open: false, noTelemetry: false, ...fields });

describe('DeviceRecordingsTest', () => {
  it('reads the recordings page the device serves', () => {
    const view = parseRecordings({
      limit: { value: '8589934592', options: [{ value: '2147483648', label: '2 GB' }], used: '2.0 KB used' },
      cameras: [{ name: 'cam2', recording: true, note: 'Recording · 2.0 KB on disk' }],
      recorded: [{ name: 'cam2', status: { level: 1, text: 'storage full' }, summary: '1 recording', full: true, usage: '2.0 KB on this camera', segments: [{ at: 'a', started: 's', size: '2.0 KB', url: '/v', open: true }] }],
    });
    expect(view?.limit.options).toEqual([{ value: '2147483648', label: '2 GB' }]);
    expect(view?.cameras[0]).toEqual({ name: 'cam2', recording: true, note: 'Recording · 2.0 KB on disk' });
    expect(view?.recorded[0]).toMatchObject({ full: true, status: { level: 1, text: 'storage full' } });
    expect(view?.recorded[0]?.segments[0]).toEqual({ at: 'a', started: 's', duration: '', size: '2.0 KB', url: '/v', open: true, noTelemetry: false });
    expect(parseRecordings({ cameras: [] })).toBeNull();
  });

  it('only a smaller limit asks before it is saved', () => {
    expect(lowersLimit('8589934592', '2147483648')).toBe(true);
    expect(lowersLimit('8589934592', '17179869184')).toBe(false);
    expect(lowersLimit('', '2147483648')).toBe(false);
  });

  it('a segment is deleted by its start, escaped', () => {
    expect(segmentPath('cam2', segment({}))).toBe('/api/recordings/cam2?start=2026-10-18T10%3A00%3A00%2B04%3A00');
    expect(recordPath('cam2')).toBe('/api/streams/cam2/record');
  });

  it('a segment says what it is missing and when it can be had', () => {
    expect(segmentDetail(segment({}))).toEqual(['10 min', '1.0 MB']);
    expect(segmentDetail(segment({ duration: '', noTelemetry: true, open: true }))).toEqual(['1.0 MB', 'no flight data', 'downloadable once it closes']);
  });
});
