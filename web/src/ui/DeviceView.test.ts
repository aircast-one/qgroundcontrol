import { describe, expect, it } from 'vitest';
import { NO_ANSWER, deviceAnswer, deviceReading, deviceRefusal, resolveDeviceUrl } from '@/ui/DeviceView';

describe('DeviceViewTest', () => {
  it('a device URL is the path itself on the device, and the device address anywhere else', () => {
    expect(resolveDeviceUrl('', '/api/views/aircraft')).toBe('/api/views/aircraft');
    expect(resolveDeviceUrl('http://192.168.1.100', '/api/logs')).toBe('http://192.168.1.100/api/logs');
    expect(resolveDeviceUrl('http://falcon.local:8080', '/fly/')).toBe('http://falcon.local:8080/fly/');
  });

  it('a page reads as the device it came from and the view the device last sent', () => {
    expect(deviceReading({ kind: 'object', page: 'aircraft', origin: 'http://falcon.local', host: 'falcon.local', answered: true, view: { headline: 'Ready' } })).toEqual({
      origin: 'http://falcon.local',
      host: 'falcon.local',
      answered: true,
      view: { headline: 'Ready' },
    });
    expect(deviceReading({ origin: '', host: '', answered: false, view: null })).toEqual({ origin: '', host: '', answered: false, view: null });
    expect(deviceReading(null).origin).toBe('');
  });

  it('a refusal names the field the device blamed, and silence is the one no-answer', () => {
    expect(deviceRefusal({ ok: true, result: {} })).toBeNull();
    expect(deviceRefusal({ ok: false, reason: 'baud must be a number', field: 'baud' })).toEqual({ message: 'baud must be a number', field: 'baud' });
    expect(deviceRefusal({ ok: false, reason: 'The device did not answer.', field: '' })).toBe(NO_ANSWER);
    expect(deviceRefusal(null)).toBe(NO_ANSWER);
    expect(deviceAnswer({ ok: false, reason: 'Name it first', field: 'name', result: { error: 'Name it first' } }).result).toEqual({ error: 'Name it first' });
  });
});
