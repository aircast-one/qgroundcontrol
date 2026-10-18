import { describe, expect, it } from 'vitest';
import { eventData, refusalOf, resolveDeviceUrl, splitEvents } from '@/ui/DeviceView';

describe('DeviceViewTest', () => {
  it('a device URL is the path itself on the device, and the device address anywhere else', () => {
    expect(resolveDeviceUrl('', '/api/views/aircraft')).toBe('/api/views/aircraft');
    expect(resolveDeviceUrl('http://192.168.1.100', '/api/views/aircraft')).toBe('http://192.168.1.100/api/views/aircraft');
    expect(resolveDeviceUrl('http://falcon.local:8080', '/qr?d=x')).toBe('http://falcon.local:8080/qr?d=x');
  });

  it('a view stream is read event by event, across chunks, skipping heartbeats', () => {
    const first = splitEvents('data: {"a":1}\n\nevent: alive\ndata: \n\ndata: {"a"');
    expect(first.events).toEqual(['{"a":1}']);
    const second = splitEvents(`${first.rest}:2}\n\n`);
    expect(second.events).toEqual(['{"a":2}']);
    expect(second.rest).toBe('');
    expect(splitEvents('data: x\r\n\r\n').events).toEqual(['x']);
    expect(eventData('event: alive\ndata: ')).toBeNull();
  });

  it('a refusal names the field the device blamed', () => {
    expect(refusalOf(400, { error: 'baud must be a number', field: 'baud' })).toEqual({ message: 'baud must be a number', field: 'baud' });
    expect(refusalOf(502, null)).toEqual({ message: 'The device refused (502).', field: '' });
  });
});
